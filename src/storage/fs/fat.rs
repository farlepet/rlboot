extern crate alloc;

use alloc::boxed::Box;
use alloc::rc::{Rc, Weak};
use alloc::vec;
use alloc::vec::Vec;
use core::any::Any;
use core::cell::RefCell;
use core::mem;
use core::fmt::Write;
use core::ptr;

use crate::errors::ErrorCode;
use crate::storage::{block::BlockDevice, fs::FileAttribute};
use crate::io::output;
use super::{File, Filesystem};


pub struct FATFilesystem {
    offset: isize, //< Offset of filesystem into block device

    sector_size: usize,    //< Number of bytes per sector
    cluster_size: usize,   //< Number of bytes per cluster
    fat_offset: isize,   //< Offset into filesystem of first FAT
    fat_size: usize,     //< Size of FAT in bytes
    data_offset: isize,  //< Offset info filesystem of first data cluster

    rootdir: FATFile, //< FATFile representing root directory

    cache: Rc<RefCell<FATFilesystemCache>>, //< FAT cluster cache

    block: Rc<RefCell<dyn BlockDevice>>, //< Underlying block device

    rc: Weak<RefCell<Self>>,
}

impl Filesystem for FATFilesystem {
    fn get_root(&self) -> &dyn super::File {
        &self.rootdir
    }

    fn find_file(&self, start_dir: Option<&dyn File>, path: &str) -> Result<Box<dyn File>, ErrorCode> {
        let mut dir: FATFile = if path.starts_with('/') || start_dir.is_none() {
            self.rootdir.clone()
        } else {
            /* Not ideal, but is fine while we only have one FS */
            start_dir.unwrap().as_any().downcast_ref::<FATFile>().unwrap().clone()
        };

        let mut cpath = path;

        /* Theoretically this part should not be handled here */
        while cpath.contains('/') {
            let (dirname, path) = cpath.split_at(cpath.find('/').unwrap());
            cpath = &path[1..];

            let res = self.find_file(Some(&dir), dirname);

            dir = res?.as_any().downcast_ref::<FATFile>().unwrap().clone();

            if (dir.attr & FileAttribute::Directory as u32) == 0 {
                return Err(ErrorCode::FileNotFound)
            }
        }

        if cpath.len() > 11 {
            /* Long filenames not yet supported */
            return Err(ErrorCode::FileUnsupported);
        }

        let mut dent_off = dir.first_cluster;

        while dent_off > 0 {
            let buf = match self.block.borrow().read(dent_off, self.cluster_size) {
                Ok(data) => data,
                Err(err) => return Err(err)
            };

            for i in 0..(self.cluster_size / mem::size_of::<FATDataDirent>()) {
                let dirent: FATDataDirent = unsafe { ptr::read(buf.as_ptr().add(i * mem::size_of::<FATDataDirent>()) as *const _) };

                if (dirent.filename[0] == b'\0') || ((dirent.attr & FATDirentAttr::VolumeLabel as u8) != 0) {
                    continue;
                }

                if Self::fat_filename_eq(&dirent.filename, cpath) {
                    return Ok(Box::new(self.populate_file(dirent)));
                }
            }

            match self.get_next_cluster(dent_off)? {
                Some(off) => dent_off = off,
                None      => return Err(ErrorCode::FileNotFound)
            }

        }


        Err(ErrorCode::FileNotFound)
    }


}

impl FATFilesystem {
    pub fn init(blockdev: &Rc<RefCell<dyn BlockDevice>>, offset: isize) -> Result<Rc<RefCell<Self>>, ErrorCode> {
        let bs = blockdev.borrow().read(offset, 512)?;
        assert_eq!(bs.len(), 512);
        let bs: FATDataBootsector = unsafe { core::ptr::read(bs.as_ptr() as *const _) };

        let sector_size = bs.bytes_per_sector as usize;
        let fat_offset  = bs.reserved_sectors as isize * sector_size as isize;
        let fat_size    = bs.sectors_per_fat as usize * sector_size as usize;
        let root_first_cluster = fat_offset + (fat_size * bs.fat_copies as usize) as isize;
        let root_size          = bs.root_dir_entries as usize * mem::size_of::<FATDataDirent>();

        Ok(Rc::new_cyclic(|me| {
            RefCell::new(FATFilesystem {
                offset,
                sector_size,
                cluster_size: bs.sectors_per_cluster as usize * sector_size,
                fat_offset,
                fat_size,
                data_offset: root_first_cluster + root_size as isize,
                rootdir: FATFile {
                    first_cluster: root_first_cluster,
                    size: root_size,
                    attr: FileAttribute::Directory as u32,
                    fs: me.clone()
                },
                cache: Rc::new(RefCell::new(FATFilesystemCache::new(4))),

                block: Rc::clone(blockdev),
                rc: me.clone(),
            })
        }))
    }

    fn populate_file(&self, dirent: FATDataDirent) -> FATFile {
        let mut attr: u32 = 0;

        if (dirent.attr & FATDirentAttr::Directory as u8) != 0 {
            attr |= FileAttribute::Directory as u32;
        } else {
            /* Only support files and directories for now */
            attr |= FileAttribute::File as u32;
        }

        FATFile {
            first_cluster: self.data_offset + (dirent.start_cluster as isize - 2) * self.cluster_size as isize,
            size: dirent.filesize as usize,
            attr,
            fs: self.rc.clone()
        }
    }

    fn read_fat_data(&self, off: usize, sz: usize) -> Result<u32, ErrorCode> {
        let read_addr = off - (off % self.sector_size);

        let mut cache = self.cache.borrow_mut();

        let buf = match cache.find(read_addr) {
            Some(data) => data,
            None       => {
                match self.block.borrow().read(read_addr as isize, self.cluster_size) {
                    Ok(data) => {
                        cache.add(read_addr, data.clone());
                        data
                    },
                    Err(err) => return Err(err)
                }
            }
        };

        let pos = off - read_addr;
        let value = u32::from_le_bytes(buf[pos..(pos+4)].try_into().unwrap());

        match sz {
            1 => Ok(value & 0x000000FF),
            2 => Ok(value & 0x0000FFFF),
            3 => Ok(value & 0x00FFFFFF),
            4 => Ok(value),
            _ => Err(ErrorCode::Unspecified)
        }
    }

    fn get_fat_entry(&self, cluster: usize) -> Result<u32, ErrorCode> {
        /* TODO: Support FAT16/FAT32 */
        let offset = self.fat_offset as usize + ((cluster * 3) / 2);

        let mut entry = if (offset % self.cluster_size) == (self.cluster_size - 1) {
            self.read_fat_data(offset, 1)? |
            self.read_fat_data(offset + 1, 1)? << 8
        } else {
            self.read_fat_data(offset, 2)?
        };

        if (cluster & 0x01) != 0 {
            entry >>= 4;
        }

        Ok(entry & 0x0FFF)
    }

    fn get_next_cluster(&self, cluster: isize) -> Result<Option<isize>, ErrorCode> {
        if cluster < self.data_offset {
            if cluster < self.rootdir.first_cluster {
                println!("get_next_cluster: {:5x} is below root directory", cluster);
                return Err(ErrorCode::OutOfBounds);
            }

            return if (cluster + self.cluster_size as isize) < self.data_offset {
                /* Root directory is contiguous */
                Ok(Some(cluster + self.cluster_size as isize))
            } else {
                /* End of root directory */
                Ok(None)
            }
        }

        let cluster_num = ((cluster - self.data_offset) / self.cluster_size as isize) + 2;
        let fat_entry = self.get_fat_entry(cluster_num as usize)?;

        if (0x002..=0xff0).contains(&fat_entry) {
            Ok(Some(self.data_offset + ((fat_entry as isize - 2) * self.cluster_size as isize)))
        } else {
            Ok(None)
        }
    }

    fn fat_filename_eq(fat_name: &[u8; 11], filename: &str) -> bool {
        let mut fidx = 0;

        for &ch in filename.as_bytes() {
            if fidx >= 11 {
                /* Requested filename longer than FAT filename */
                return false;
            }
            if ch == b'.' {
                while (fidx < 11) && (fat_name[fidx] == b' ') {
                    fidx += 1;
                }
                continue;
            }
            if fat_name[fidx] != ch {
                return false;
            }
            fidx += 1;
        }

        while fidx < 11 {
            if fat_name[fidx] != b' ' {
                /* FAT filename longer than requested filename */
                return false;
            }
            fidx += 1;
        }
        true
    }

    pub fn get_cluster_size(&self) -> usize {
        self.cluster_size
    }
}

#[derive(Clone)]
pub struct FATFile {
    first_cluster: isize, //< Offset into filesystem of first data cluster

    size: usize, //< File size

    attr: u32,   //< File attributes

    fs: Weak<RefCell<FATFilesystem>>,
}

impl File for FATFile {
    fn get_size(&self) -> usize {
        self.size
    }

    fn get_attr(&self) -> u32 {
        self.attr
    }

    fn read(&self, mut offset: isize, size: usize) -> Result<Vec<u8>, ErrorCode> {
        if ((self.attr & FileAttribute::Directory as u32) == 0) && (offset as usize + size) > self.size {
            println!("FATFile: Attempt to read past end of file");
            return Err(ErrorCode::OutOfBounds);
        }

        let fs = self.fs.upgrade().expect("Could not upgrade FATFile::fs");

        let mut clust = self.first_cluster;

        let cluster_size = fs.borrow().get_cluster_size();
        let block = fs.borrow().block.clone();

        while offset as usize > cluster_size {
            clust = match fs.borrow().get_next_cluster(clust) {
                Ok(Some(cluster)) => cluster,
                Ok(None)          => return Err(ErrorCode::Unspecified),
                Err(err)          => return Err(err)
            };
            offset -= cluster_size as isize;
        }

        let mut read_data: Vec<u8> = vec!();

        let mut pos = 0;
        while pos < size {
            let data = match block.borrow().read(clust, cluster_size) {
                Ok(res)  => res,
                Err(err) => return Err(err)
            };

            if (size - pos) <= (cluster_size - offset as usize) {
                read_data.extend_from_slice(&data[offset as usize..(size-pos)]);
            } else {
                read_data.extend_from_slice(&data[offset as usize..(cluster_size - offset as usize)]);

                clust = match fs.borrow().get_next_cluster(clust) {
                    Ok(Some(cluster)) => cluster,
                    Ok(None)          => return Err(ErrorCode::Unspecified),
                    Err(err)          => return Err(err)
                };
            }

            offset = 0;
            pos += cluster_size;
        }

        Ok(read_data)
    }

    fn close(&self) {

    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}


struct FATFilesystemCacheItem {
    cluster: usize, //< Offset of cached cluster into filesystem
    rank: u8,       //< Rank of entry, indicating which was last used
    data: Vec<u8>
}

struct FATFilesystemCache {
    items: Vec<FATFilesystemCacheItem>
}

impl FATFilesystemCache {
    fn new(count: usize) -> FATFilesystemCache {
        let mut ffc = FATFilesystemCache {
            items: vec!()
        };

        for _ in 0..count {
            ffc.items.push(FATFilesystemCacheItem {
                cluster: usize::MAX,
                rank: 0,
                data: vec!()
            })
        }

        ffc
    }

    fn touch(&mut self, index: usize) {
        let old_rank = self.items[index].rank;
        for item in self.items.iter_mut() {
            if item.rank < old_rank {
                item.rank += 1;
            }
        }
        self.items[index].rank = 0;
    }

    fn find(&mut self, cluster: usize) -> Option<Vec<u8>> {
        for idx in 0..self.items.len() {
            if self.items[idx].cluster == cluster {
                self.touch(idx);
                return Some(self.items[idx].data.clone());
            }
        }

        None
    }

    fn add(&mut self, cluster: usize, data: Vec<u8>) {
        let mut entry = 0;
        /* Find the highest-ranked (oldest) entry */
        for idx in 1..self.items.len() {
            if self.items[idx].rank > self.items[entry].rank {
                entry = idx;
            }
        }

        self.items[entry].cluster = cluster;
        self.items[entry].data    = data;
    }
}

#[allow(dead_code)]
#[repr(C, packed(1))]
struct FATDataBootsectorExtFAT12 {
    drive_number: u8,       //< BIOS drive number
    _reserved: u8,
    ext_signature: u8,      //< Extended signature, if 0x28 or 0x29, the following two fields are valid
    serial_number: u32,     //< Volume serial number
    volume_label: [u8; 11], //< Volume label
    fs_type: [u8; 8],       //< FS type ["FAT12   ", "FAT16   ", "FAT     ", "\0"]
    code: [u8; 444]         //< Boot sector code
}

#[allow(dead_code)]
#[repr(C, packed(1))]
struct FATDataBootsectorExtFAT32 {
    sectors_per_fat_big: u32, //< Sectors per track
    mirror_flags: u16,        //< FAT mirror flags
    fs_version: u16,          //< Filesystem version
    root_cluster: u32,        //< First cluster of root directory
    info_sector: u16,         //< Filesystem information cluster
    bs_backup: u16,           //< Boot sector backup sector
    _reserved0: [u8; 11],
    drive_number: u8,         //< BIOS drive number
    _reserved1: u8,
    ext_signature: u8,        //< Extended signature, if 0x28 or 0x29, the following two fields are valid
    serial_number: u32,       //< Volume serial number
    volume_label: [u8; 11],   //< Volume label
    fs_type: [u8; 8],         //< FS type ("FAT32   ")
    code: [u8; 416]           //< Boot sector code
}

#[allow(dead_code)]
#[repr(C, packed(1))]
union FATDataBootsectorExt {
    fat12: mem::ManuallyDrop<FATDataBootsectorExtFAT12>,
    fat32: mem::ManuallyDrop<FATDataBootsectorExtFAT32>
}

#[allow(dead_code)]
#[repr(C, packed(1))]
struct FATDataBootsector {
    _jump: [u8; 3],          //< Jump to code
    name:  [u8; 8],          //< OEM name, or name of formatting utility
    bytes_per_sector: u16,   //< Bytes per sector
    sectors_per_cluster: u8, //< Sectors per cluster
    reserved_sectors: u16,   //< Number of reserved sectors
    fat_copies: u8,          //< Number of FAT copies
    root_dir_entries: u16,   //< Number of root directory entries
    total_sectors: u16,      //< Total number of sectors in the filesystem, overriden by `total_sectors_big`
    media_desc_type: u8,     //< Media descriptor type
    sectors_per_fat: u16,    //< Sectors per FAT
    sectors_per_track: u16,  //< Sectors per track
    heads: u16,              //< Number of heads
    /* @note FAT12 need not have all the data following this point (apart from a
     * 16-bit hidden_sectors), but most modern implementations will. */
    hidden_sectors: u32,     //< Number of hidden sectors
    total_sectors_big: u32,  //< Total number sectors in the filesystem

    ext: FATDataBootsectorExt, //< FAT12/16/32-specific data

    stage2_map_sector: u16,  //< rlboot-specific: First sector of stage2 sector map
    stage2_addr: u16,        //< rlboot-specific: Address at which to load stage2
    signature: u16           //< Boot sector signature: 0x55, 0xAA
}
const _FAT_BOOTSECTOR_SZ_TEST: [u8; 512] = [0; mem::size_of::<FATDataBootsector>()];


#[repr(u8)]
#[allow(dead_code)]
enum FATDirentAttr {
    ReadOnly    = (1 << 0),
    Hidden      = (1 << 1),
    System      = (1 << 2),
    VolumeLabel = (1 << 3),
    Directory   = (1 << 4),
    Archive     = (1 << 5),
    Device      = (1 << 6),
    Reserved    = (1 << 7)
}

#[allow(dead_code)]
#[repr(C, packed(1))]
struct FATDataDirent {
    filename: [u8; 11],  //< Short filename
    attr: u8,            //< Attributes
    _reserved: [u8; 10], //< Reserved: Used for VFAT, not yet supported
    time: u16,           //< Modification time, in FAT time format
    date: u16,           //< Modification date, in FAT date format
    start_cluster: u16,  //< First cluster of file, 0 if file is empty
    filesize: u32        //< Size of file in bytes
}
const _FAT_DIRENT_SZ_TEST: [u8; 32] = [0; mem::size_of::<FATDataDirent>()];

