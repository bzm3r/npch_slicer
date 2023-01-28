use lopdf::Document;
use serde::Deserialize;
use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::ops::Sub;
use std::path::PathBuf;
use std::process::Command;
use std::slice::Iter;
use thiserror::Error;

#[derive(Debug, Deserialize)]
struct RawSliceRequest {
    description: String,
    start_page: u32,
    end_page: u32,
}

#[derive(Error, Debug)]
enum FromRawError {
    #[error("Invalid page range for {description:?}: {start_page:?}, {end_page:?}")]
    InvalidPageRange {
        description: String,
        start_page: u32,
        end_page: u32,
    },
    #[error("empty page range for {description:?} (start == end)")]
    EmptyPageRange { description: String },
}

impl TryFrom<RawSliceRequest> for SliceRequest {
    type Error = FromRawError;

    fn try_from(record: RawSliceRequest) -> Result<Self, Self::Error> {
        let RawSliceRequest {
            description,
            start_page,
            end_page,
        } = record;
        match start_page.cmp(&end_page) {
            Ordering::Less => Ok(SliceRequest {
                description,
                start_page,
                end_page,
                pages: BTreeSet::from_iter(start_page..end_page),
            }),
            Ordering::Equal => Err(Self::Error::EmptyPageRange { description }),
            Ordering::Greater => Err(Self::Error::InvalidPageRange {
                description,
                start_page,
                end_page,
            }),
        }
    }
}

#[derive(Debug)]
struct SliceRequest {
    description: String,
    start_page: u32,
    end_page: u32,
    pages: BTreeSet<u32>,
}

struct SliceRequests {
    individuals: Vec<SliceRequest>,
    #[allow(unused)]
    required_pages: BTreeSet<u32>,
}

impl SliceRequests {
    fn new(individuals: Vec<SliceRequest>) -> SliceRequests {
        let mut required_pages = BTreeSet::new();

        for slice_request in individuals.iter() {
            for pg in slice_request.start_page..slice_request.end_page {
                required_pages.insert(pg);
            }
        }

        SliceRequests {
            individuals,
            required_pages,
        }
    }

    #[allow(unused)]
    fn unnecessary_pages(&self, all_pages: &BTreeSet<u32>) -> BTreeSet<u32> {
        all_pages.sub(&self.required_pages)
    }

    fn iter(&self) -> Iter<'_, SliceRequest> {
        self.individuals.iter()
    }
}

fn slice() -> SliceRequests {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path("./inputs/npch_slicer.csv")
        .unwrap();

    let raw_slice_requests = reader
        .deserialize()
        .collect::<Result<Vec<RawSliceRequest>, _>>()
        .unwrap();

    let individual_slice_requests = raw_slice_requests
        .into_iter()
        .map(SliceRequest::try_from)
        .collect::<Result<Vec<SliceRequest>, _>>()
        .unwrap();

    SliceRequests::new(individual_slice_requests)
}

fn slice_guide(slice_requests: SliceRequests) {
    let document = Document::load("./inputs/npch_guide.pdf").unwrap();

    let all_pages = document
        .get_pages()
        .keys()
        .copied()
        .collect::<BTreeSet<u32>>();

    // let unnecessary_pages = slice_requests
    //     .unnecessary_pages(&all_pages)
    //     .into_iter()
    //     .collect::<Vec<u32>>();
    //
    // document.delete_pages(&unnecessary_pages);
    // let remaining_pages = document
    //     .get_pages()
    //     .keys()
    //     .copied()
    //     .collect::<BTreeSet<u32>>();

    std::fs::create_dir_all("./outputs/unoptimized/").unwrap();
    std::fs::create_dir_all("./outputs/optimized/").unwrap();

    for slice_request in slice_requests.iter() {
        let required_deletions = all_pages
            .sub(&slice_request.pages)
            .into_iter()
            .collect::<Vec<u32>>();
        let mut slice_pdf = document.clone();
        slice_pdf.delete_pages(&required_deletions);
        slice_pdf.prune_objects();
        slice_pdf
            .save(format!(
                "./outputs/unoptimized/{}.pdf",
                slice_request.description
            ))
            .unwrap();

        shrink(&slice_request.description);
    }
}

fn shrink(pdf_name: &str) {
    let input_path = PathBuf::from(format!("./outputs/unoptimized/{pdf_name}.pdf"));
    let pre_shrink_size = input_path.metadata().unwrap().len() as f32;

    let output_path = PathBuf::from(format!("./outputs/optimized/{pdf_name}.pdf"));
    let image_resolution = 60;
    Command::new("gswin64")
        .arg("-dBATCH")
        .arg("-dNOPAUSE")
        .arg("-q")
        .arg("-dCompatibilityLevel=1.4")
        .arg("-dPDFSETTINGS=/screen")
        .arg(format!("-r{image_resolution}"))
        .arg("-sDEVICE=pdfwrite")
        .arg(format!("-sOutputFile={}", output_path.display()))
        .arg(&input_path)
        .output()
        .unwrap();

    let post_shrink_size = output_path.metadata().unwrap().len() as f32;

    println!(
        "Shrunk {}: {:.2}MB -> {:.2}MB",
        pdf_name,
        pre_shrink_size / 1e6,
        post_shrink_size / 1e6,
    );
}

fn main() {
    let slice_requests = slice();
    slice_guide(slice_requests);
}
