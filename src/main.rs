use lopdf::Document;
use serde::Deserialize;
use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::error::Error;
use std::ops::Sub;
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

fn slice() -> Result<SliceRequests, Box<dyn Error>> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path("./inputs/npch_slicer.csv")?;

    let raw_slice_requests = reader
        .deserialize()
        .collect::<Result<Vec<RawSliceRequest>, _>>()?;

    let individual_slice_requests = raw_slice_requests
        .into_iter()
        .map(SliceRequest::try_from)
        .collect::<Result<Vec<SliceRequest>, _>>()?;

    Ok(SliceRequests::new(individual_slice_requests))
}

fn slice_guide(slice_requests: SliceRequests) -> Result<(), Box<dyn Error>> {
    let document = Document::load("./inputs/npch_guide.pdf")?;

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

    std::fs::create_dir_all("./outputs/")?;

    for slice_request in slice_requests.iter() {
        let required_deletions = all_pages
            .sub(&slice_request.pages)
            .into_iter()
            .collect::<Vec<u32>>();
        let mut slice_pdf = document.clone();
        slice_pdf.delete_pages(&required_deletions);
        slice_pdf.save(format!("./outputs/{}.pdf", slice_request.description))?;
    }

    Ok(())
}

fn main() {
    let slice_requests = slice().unwrap();
    slice_guide(slice_requests).unwrap();
}
