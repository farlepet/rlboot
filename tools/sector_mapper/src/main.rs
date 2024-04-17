pub mod fat;
use core::slice;
use std::{
    error::Error,
    fs::File,
    io::Write,
    mem::{self, MaybeUninit},
    path::Path,
    env
};

use fat::{FAT, FatDataBootsector};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {
        usage();
        return Err("Not enough arguments".into());
    }
    let stage: u8 = args[1].parse()?;
    if (stage < 1) || (stage > 2) {
        usage();
        return Err("Invalid stage".into());
    }

    let fatfs = FAT::open(Path::new(&args[2]), stage == 2)?;
    match stage {
        1 => stage1(&fatfs, &args),
        2 => stage2(&fatfs, &args),
        _ => Err("Invalid stage - broken check logic".into())
    }
}

fn usage() {
    eprintln!("USAGE: ");
    eprintln!("  sector_mapper 1 <floppy image> <stage 2 filename> <map file>");
    eprintln!("  sector_mapper 2 <floppy image> <map file>");
}

fn stage1(fs: &FAT, args: &Vec<String>) -> Result<(), Box<dyn Error>> {
    if args.len() != 5 {
        usage();
        return Err("stage1: Incorrect number of arguments".into());
    }

    let mut map_file = File::create(Path::new(&args[4]))?;
    let s2file = fs.find_file(None, &args[3])?;

    let clusters = fs.get_file_clusters(&s2file)?;

    let mut chunks: Vec<SectorMapChunk> = vec!();
    let mut chunk = SectorMapChunk::default();
    chunk.entries[0] = SectorMapEntry {
        sector: ((clusters[0] as usize / fs.get_cluster_size()) * fs.get_sectors_per_cluster()) as u16,
        count:  1
    };
    let mut cmap = 0;

    for i in 1..clusters.len() {
        if clusters[i] != (clusters[i-1] + fs.get_cluster_size() as u32) {
            cmap += 1;
            if cmap >= SECTOR_MAP_ENTRIES {
                cmap = 0;
                chunks.push(chunk);
            }

            chunk.entries[cmap] = SectorMapEntry {
                sector: ((clusters[i] as usize / fs.get_cluster_size()) * fs.get_sectors_per_cluster()) as u16,
                count:  1
            };
        } else {
            chunk.entries[cmap].count += 1;
        }
    }
    chunks.push(chunk);

    let ptr = chunks.as_ptr().cast();
    let len = chunks.len() * mem::size_of::<SectorMapChunk>();

    let data = unsafe { slice::from_raw_parts(ptr, len) };

    if let Err(e) = map_file.write(data) {
        Err(Box::new(e))
    } else {
        Ok(())
    }
}

fn stage2(fs: &FAT, args: &Vec<String>) -> Result<(), Box<dyn Error>> {
    if args.len() != 4 {
        usage();
        return Err("stage2: Incorect number of arguments".into());
    }

    let mapfile = fs.find_file(None, &args[3])?;

    let clusters = fs.get_file_clusters(&mapfile)?;

    let mut off = mem::offset_of!(FatDataBootsector, stage2_map_sector);
    for clust in clusters {
        let real_clust = ((clust as usize / fs.get_cluster_size()) * fs.get_sectors_per_cluster()) as u16;
        if let Err(e) = fs.write_u16(off, real_clust) {
            return Err(e);
        }
        off = clust as usize + mem::offset_of!(SectorMapChunk, next_chunk);
    }

    Ok(())
}

#[repr(C)]
#[derive(Clone, Copy)]
struct SectorMapEntry {
    sector: u16,
    count:  u16,
}

const SECTOR_MAP_ENTRIES: usize = (512 - 4) / mem::size_of::<SectorMapEntry>();

#[repr(C)]
#[derive(Clone, Copy)]
struct SectorMapChunk {
    entries: [SectorMapEntry; SECTOR_MAP_ENTRIES],
    next_chunk: u16,
    _reserved: u16,
}

impl Default for SectorMapChunk {
    fn default() -> Self {
        unsafe { MaybeUninit::zeroed().assume_init() }
    }
}
