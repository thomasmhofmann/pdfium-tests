use std::time::Duration;
use std::{path::Path, time::Instant};

use clap::{command, Parser};
use human_bytes::human_bytes;
use memory_stats::memory_stats;
use pdfium_render::prelude::*;

use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::UTF8_FULL;
use comfy_table::*;

/*
use re_memory::{AccountingAllocator, MemoryUse};

#[global_allocator]
static GLOBAL: AccountingAllocator<std::alloc::System> =
    AccountingAllocator::new(std::alloc::System);
 */

#[derive(Parser)]
#[command(name = "pdfium-tests")]
#[command(version = "1.0")]
#[command(about = "Concats several PDF documents into one", long_about = None)]
struct Args {
    /// The number of the document to start with.
    #[arg(short = 's', long, default_value_t = 7000000)]
    start: u32,

    /// The number of documents to process.
    #[arg(short, long, default_value_t = 100)]
    count: u32,

    /// Whether to add a watermark or not
    #[arg(short, long, default_value = "false")]
    watermark: bool,

    /// The directory where the PDF files to process are stored.
    #[arg(short = 'd', long, default_value = ".")]
    source_directory: String,

    /// The path to the merged PDF file.
    #[arg(short, long, default_value = "merged.pdf")]
    target: String,
}

fn main() -> Result<(), PdfiumError> {
    let mut maxMem = 0;
    let args = Args::parse();

    let start = Instant::now();

    let pdfium = Pdfium::default();
    let mut document = pdfium.create_new_pdf()?;
    let font: PdfFontToken = document.fonts_mut().helvetica();

    println!("Merging {} documents", args.count);
    report_memory(&mut maxMem);


    for i in args.start..args.start + args.count {
        let file_path = format!("{}/{}.pdf", args.source_directory, i);

        if Path::new(&file_path).exists() {
            println!("Adding {}", file_path);
            let _document_to_add = match pdfium.load_pdf_from_file(&file_path, None) {
                Ok(mut doc) => {
                    println!("Imported document {}", &file_path);

                    if args.watermark {
                        watermark(font, &mut doc)?;
                    }

                    document.pages_mut().append(&doc)?;
                    report_memory(&mut maxMem);
                    Some(doc)
                }
                Err(e) => {
                    println!("Error while importing {}: {}", &file_path, e.to_string());
                    None
                }
            };
        } else {
            println!("Skipping {} as it does not exist.", &file_path);
        }
    }

    println!("Creating {}", args.target);
    report_memory(&mut maxMem);
    document.save_to_file(&args.target)?;
    report_memory(&mut maxMem);
    print_summary(args, start.elapsed(), maxMem);
    
    Ok(())
}

fn watermark(
    font: impl ToPdfFontToken + Clone,
    document: &mut PdfDocument,
) -> Result<(), PdfiumError> {
    let start = Instant::now();
    document.pages().watermark(|group, index, width, height| {
        let mut page_number = PdfPageTextObject::new(
            &document,
            format!("Page {}", index + 1),
            font.clone(),
            PdfPoints::new(14.0),
        )?;

        page_number.set_fill_color(PdfColor::GREEN)?;

        page_number.translate(
            (width - page_number.width()?) / 2.0, // Horizontally center the page number...
            height - page_number.height()?,       // ... and vertically position it at the page top.
        )?;

        group.push(&mut page_number.into())?;
        Ok(())
    })?;
    let duration = start.elapsed();
    println!(
        "Watermarking {} pages took: {:?}",
        document.pages().len(),
        duration
    );
    Ok(())
}

fn report_memory(maxMem: &mut usize) {
    if let Some(usage) = memory_stats() {
        println!();
        println!("Physical memory usage: {}", human_bytes(usage.physical_mem as f64));
        //println!("Virtual memory usage: {}", human_bytes(usage.virtual_mem as f64));
        if *maxMem < usage.physical_mem {
            *maxMem = usage.physical_mem;
        }
    } else {
        println!("Couldn't get the current memory usage :(");
    }

    /*
    let usage = MemoryUse::capture();
    match usage.resident {
        Some(bytes) => println!("Resident memory: {}", human_bytes(bytes as f64)),
        None => println!("Resident memory not available")
    };
    */
}

fn print_summary(args: Args, duration: Duration, maxMem: usize) {
    use filesize::PathExt;
    println!("Time elapsed is: {:?}", duration);

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_width(100)
        .set_header(vec!["Source", "Start", "Count", "Time Elapsed (ms)", "Max Memory", "Target File Size"])
        .add_row(vec![
            Cell::new(args.source_directory),
            Cell::new(args.start),
            Cell::new(args.count),
            Cell::new(format!("{:?}", duration)),
            Cell::new(human_bytes(maxMem as f64)),
            Cell::new(human_bytes(Path::new(&args.target).size_on_disk().unwrap() as f64)),
        ]);
    

    // Set the default alignment for the third column to right
    let column = table.column_mut(2).expect("Our table has three columns");
    column.set_cell_alignment(CellAlignment::Right);
    println!("{table}");

    println!("Target File: {}", args.target)
}
