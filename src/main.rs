use std::{path::Path, time::Instant};

use clap::{command, Parser};
use human_bytes::human_bytes;
use memory_stats::memory_stats;
use pdfium_render::prelude::*;

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

    /// The directory where the PDF files to process are stored.
    #[arg(short = 'd', long, default_value = ".")]
    source_directory: String,

    /// The path to the merged PDF file.
    #[arg(short, long, default_value = "merged.pdf")]
    target: String,
}

fn main() -> Result<(), PdfiumError> {
    let args = Args::parse();

    let start = Instant::now();

    let pdfium = Pdfium::default();
    let mut document = pdfium.create_new_pdf()?;
    let font: PdfFontToken = document.fonts_mut().helvetica();

    println!("Merging {} documents", args.count);
    report_memory();

    for i in args.start..args.start + args.count {
        let file_path = format!("{}/{}.pdf", args.source_directory, i);

        if Path::new(&file_path).exists() {
            println!("Adding {}", file_path);
            let _document_to_add = match pdfium.load_pdf_from_file(&file_path, None) {
                Ok(mut doc) => {
                    println!("Imported document {}", &file_path);
                    watermark(font, &mut doc)?;
                    document.pages_mut().append(&doc)?;
                    report_memory();
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
    report_memory();
    document.save_to_file(&args.target)?;
    report_memory();

    let duration = start.elapsed();
    println!("Time elapsed is: {:?}", duration);
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

fn report_memory() {
    if let Some(usage) = memory_stats() {
        println!();
        println!("Physical memory usage: {}", human_bytes(usage.physical_mem as f64));
        //println!("Virtual memory usage: {}", human_bytes(usage.virtual_mem as f64));
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
