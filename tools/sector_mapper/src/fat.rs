use core::slice;

use std::{
    error::Error,
    io::Read,
    mem::{self, MaybeUninit},
    fs::File,
    os::unix::fs::FileExt,
    path::Path
};

pub struct FAT {
    file: File, //< Floppy disk, or file containing floppy image

    cluster_size: usize, //< Number of bytes per cluster
    fat_offset: isize,   //< Offset into filesystem of first FAT
    fat_size: usize,     //< Size of FAT in bytes
    data_offset: isize,  //< Offset info filesystem of first data cluster

    bootsector: FatDataBootsector, //< Bootsector data

    rootdir: FATFile, //< FatFile representing root directory
}

impl FAT {
    pub fn open(path: &Path, wr: bool) -> Result<Self, Box<dyn Error>> {
        let mut fatfs = FAT {
            file: File::options().write(wr).read(true).open(path)?,

            cluster_size: 0,
            fat_offset:   0,
            fat_size:     0,
            data_offset:  0,

            bootsector: FatDataBootsector::default(),

            rootdir: FATFile {
                size: 0,
                first_cluster: 0,
                attr: 0
            }
        };

        unsafe {
            let mut bs_slice = slice::from_raw_parts_mut(&mut fatfs.bootsector as *mut FatDataBootsector as *mut u8,
                                                         mem::size_of::<FatDataBootsector>());
            if fatfs.file.read_exact(&mut bs_slice).is_err() {
                return Err("Could not read bootsector".into());
            }
        }

        if fatfs.bootsector.signature != 0xAA55 {
            let signature = fatfs.bootsector.signature;
            return Err(format!("Incorrect signatore: {:4x}", signature).into());
        }

        fatfs.cluster_size = fatfs.bootsector.sectors_per_cluster as usize * fatfs.bootsector.bytes_per_sector as usize;
        fatfs.fat_offset   = fatfs.bootsector.reserved_sectors as isize    * fatfs.bootsector.bytes_per_sector as isize;
        fatfs.fat_size     = fatfs.bootsector.sectors_per_fat as usize     * fatfs.bootsector.bytes_per_sector as usize;

        fatfs.rootdir.first_cluster = fatfs.fat_offset + (fatfs.fat_size as isize * fatfs.bootsector.fat_copies as isize);
        fatfs.rootdir.size          = fatfs.bootsector.root_dir_entries as usize * mem::size_of::<FatDataDirent>();

        fatfs.data_offset = fatfs.rootdir.first_cluster + fatfs.rootdir.size as isize;

        Ok(fatfs)
    }

    pub const fn get_cluster_size(&self) -> usize {
        self.cluster_size
    }

    pub const fn get_sectors_per_cluster(&self) -> usize {
        self.bootsector.sectors_per_cluster as usize
    }

    pub fn write_u16(&self, off: usize, value: u16) -> Result<(), Box<dyn Error>> {
        let buf = u16::to_le_bytes(value);
        if self.file.write_at(&buf, off as u64)? < 2 {
            return Err("Could not write sector map cluster number".into());
        }

        Ok(())
    }

    pub fn find_file(&self, start_dir: Option<&FATFile>, path: &str) -> Result<FATFile, Box<dyn Error>> {
        let mut dir = if path.starts_with('/') || (start_dir.is_none()) {
            self.rootdir
        } else {
            *start_dir.unwrap()
        };

        let mut cpath = path;

        while cpath.contains('/') {
            let (dirname, path) = cpath.split_at(cpath.find('/').unwrap());
            cpath = &path[1..];

            match self.find_file(Some(&dir), dirname) {
                Ok(file) => { dir = file; },
                Err(err) => { return Err(err); }
            }

            if (dir.attr & FatDirentAttr::Directory as u8) == 0 {
                return Err("Path contains file that is not a directory".into());
            }
        }

        if cpath.len() > 11 {
            /* Long filenames not yet supported */
            return Err("Long filenames not supported".into());
        }

        let mut dent_off = dir.first_cluster;

        while dent_off > 0 {
            let mut buf = vec![0_u8; self.cluster_size];
            if self.file.read_at(&mut buf, dent_off as u64)? < self.cluster_size {
                return Err(format!("Could not read cluster at {:04x}", dent_off).into());
            }

            for i in 0..(self.cluster_size / mem::size_of::<FatDataDirent>()) {
                let dirent: FatDataDirent = unsafe { std::ptr::read(buf.as_ptr().add(i * mem::size_of::<FatDataDirent>()) as *const _) };

                if (dirent.filename[0] == b'\0') || ((dirent.attr & FatDirentAttr::VolumeLabel as u8) != 0) {
                    continue;
                }

                if Self::fat_filename_eq(&dirent.filename, cpath) {
                    return Ok(FATFile {
                        first_cluster: self.data_offset + (dirent.start_cluster as isize - 2) * self.cluster_size as isize,
                        size: dirent.filesize as usize,
                        attr: dirent.attr
                    });
                }
            }

            match self.get_next_cluster(dent_off)? {
                Some(off) => dent_off = off,
                None      => return Err("File not found".into()),
            }
        }

        Err("File not found".into())
    }

    pub fn get_file_clusters(&self, file: &FATFile) -> Result<Vec<u32>, Box<dyn Error>> {
        let mut clust = file.first_cluster;
        let mut pos = 0;
        let mut clusters: Vec<u32> = vec!();

        while pos < file.size {
            if clust == 0 {
                return Err("Unexpected end of cluster chain before end of file".into());
            }

            clusters.push(clust as u32);
            pos += self.cluster_size;
            match self.get_next_cluster(clust)? {
                Some(cl) => clust = cl,
                None     => clust = 0
            }
        }

        Ok(clusters)
    }

    fn get_fat_entry(&self, cluster: isize) -> Result<u32, Box<dyn Error>> {
        /* TODO: Support FAT16/FAT32 */
        let offset = self.fat_offset + ((cluster * 3) / 2);

        let mut buf = [0_u8; 2];
        if self.file.read_at(&mut buf, offset as u64)? < 2 {
            return Err(format!("Could not read FAT entry for cluster {}", cluster).into());
        }
        let mut entry = u16::from_le_bytes(buf);
        if (cluster & 0x01) != 0 {
            entry >>= 4;
        }

        Ok(entry as u32 & 0x0FFF)
    }

    fn get_next_cluster(&self, cluster: isize) -> Result<Option<isize>, Box<dyn Error>> {
        if cluster < self.data_offset {
            if cluster < self.rootdir.first_cluster {
                return Err(format!("get_next_cluster: {:5x} is below root directory", cluster).into());
            }

            if (cluster + self.cluster_size as isize) < self.data_offset {
                /* Root directory is contiguous */
                return Ok(Some(cluster + self.cluster_size as isize));
            } else {
                /* End of root directory */
                return Ok(None);
            }
        }

        let cluster_num = ((cluster - self.data_offset) / self.cluster_size as isize) + 2;
        let fat_entry = self.get_fat_entry(cluster_num)?;

        if (fat_entry >= 0x002) && (fat_entry <= 0xff0) {
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
}

#[derive(Clone, Copy)]
pub struct FATFile {
    size: usize,          //< File size in bytes
    first_cluster: isize, //< Offset into filesystem of first data cluster
    attr: u8,             //< FAT file attributes
}


#[repr(C, packed(1))]
struct FatDataBootsectorExtFAT12 {
    drive_number: u8,       //< BIOS drive number
    _reserved: u8,
    ext_signature: u8,      //< Extended signature, if 0x28 or 0x29, the following two fields are valid
    serial_number: u32,     //< Volume serial number
    volume_label: [u8; 11], //< Volume label
    fs_type: [u8; 8],       //< FS type ["FAT12   ", "FAT16   ", "FAT     ", "\0"]
    code: [u8; 444]         //< Boot sector code
}

#[repr(C, packed(1))]
struct FatDataBootsectorExtFAT32 {
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

#[repr(C, packed(1))]
union FatDataBootsectorExt {
    fat12: mem::ManuallyDrop<FatDataBootsectorExtFAT12>,
    fat32: mem::ManuallyDrop<FatDataBootsectorExtFAT32>
}

#[repr(C, packed(1))]
pub struct FatDataBootsector {
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

    ext: FatDataBootsectorExt, //< FAT12/16/32-specific data

    pub stage2_map_sector: u16, //< rlboot-specific: First sector of stage2 sector map
    stage2_addr: u16,           //< rlboot-specific: Address at which to load stage2
    signature: u16              //< Boot sector signature: 0x55, 0xAA
}
const _FAT_BOOTSECTOR_SZ_TEST: [u8; 512] = [0; mem::size_of::<FatDataBootsector>()];

impl Default for FatDataBootsector {
    fn default() -> Self {
        unsafe { MaybeUninit::zeroed().assume_init() }
    }
}

#[repr(u8)]
#[allow(dead_code)]
enum FatDirentAttr {
    ReadOnly    = (1 << 0),
    Hidden      = (1 << 1),
    System      = (1 << 2),
    VolumeLabel = (1 << 3),
    Directory   = (1 << 4),
    Archive     = (1 << 5),
    Device      = (1 << 6),
    Reserved    = (1 << 7)
}

#[repr(C, packed(1))]
struct FatDataDirent {
    filename: [u8; 11],  //< Short filename
    attr: u8,            //< Attributes
    _reserved: [u8; 10], //< Reserved: Used for VFAT, not yet supported
    time: u16,           //< Modification time, in FAT time format
    date: u16,           //< Modification date, in FAT date format
    start_cluster: u16,  //< First cluster of file, 0 if file is empty
    filesize: u32        //< Size of file in bytes
}
const _FAT_DIRENT_SZ_TEST: [u8; 32] = [0; mem::size_of::<FatDataDirent>()];

