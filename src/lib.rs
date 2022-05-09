use std::path::Path;

use djanco::*;
use djanco::database::*;
use djanco::log::*;
use djanco::csv::*;
use djanco::objects::*;

use djanco::time::Duration;
use djanco_ext::*;

const SELECTION_SIZE: usize = 1020;
const HEADERS: [&'static str; 3] = ["pid", "path", "hash_id"];

// Seedds for different selections
const SEED_ALL: u128 = 1;
const SEED_100LOC_7D_10C: u128 = 2;
const SEED_1000LOC_180D_100C: u128 = 3;

pub fn _map_to_output_format(project: &ItemWithData<Project>) -> Option<Vec<(ProjectId, String, SnapshotId)>> {
    let project_id = project.id();

    // Get default branch, if it's not there, skip and print warning.
    let default_branch = project.default_branch();
    if default_branch.is_none() {
        eprintln!("WARNING: no default branch found for project {}, skipping.", project_id);
        return None
    }
    let default_branch = default_branch.unwrap();
    let default_branch_path = format!("refs/heads/{}", default_branch);

    // Get all heads, if it's not there, skip and print warning.
    let heads = project.heads_with_data();
    if heads.is_none() {
        eprintln!("WARNING: no heads found for project {}, skipping.", project_id);
        return None        
    }
    let heads = heads.unwrap();

    // Get head of the default branch if it's not there, skip and print warning, ort if there are several, also print warning.
    let default_heads: Vec<ItemWithData<Head>> = heads.into_iter()
        .filter(|head| head.name() == default_branch_path)
        .collect();
    if default_heads.len() == 0 {
        eprintln!("WARNING: no default head found for project {}, skipping.", project_id);
        return None
    }
    if default_heads.len() > 1 {
        eprintln!("WARNING: multiple ({}) default heads found for project {}, using whichever is first.", default_heads.len(), project_id);
    }
    let head = default_heads[0].clone();
    
    // Get commit from the head, or warn.
    let head_commit = head.commit_with_data();
    if head_commit.is_none() {
        eprintln!("WARNING: no commit found at default head found for project {} (for commit_id: {}), skipping.", project_id, head.commit_id());
        return None
    }
    let head_commit = head_commit.unwrap();

    // Get thge tree, stream it as a stream of changes (path_id, snapshot_id), convert to specified output format
    let head_tree = head_commit.tree_with_data();    
    let changes = head_tree.changes_with_data().into_iter()
        // Map to path_id, path and snapshot id. Path id is only there for reporting warnings later.
        .map(|change| (change.path_id(), change.path(), change.snapshot_id()))
        // Remove Options: warn if options appear.
        .flat_map(|(path_id, path, snapshot_id)| {
            if path.is_none() {
                eprintln!("WARNING: path not found for project {} for path id {}, skipping this change.", project_id, path_id);                
                return None
            }
            /* THIS IS NORMAL, MEANS FILE HAS BEEN DELETED */
            if snapshot_id.is_none() {
                eprintln!("WARNING: snapshot id not found for project {} for path id {}, skipping this change.", project_id, path_id);
                return None
            }
            
            Some((project_id.clone(), path.unwrap().location(), snapshot_id.unwrap()))
        })
        .collect::<Vec<(ProjectId, String, SnapshotId)>>();

    // Yay, done!
    Some(changes)
}

pub fn map_to_output_format(project: ItemWithData<Project>) -> Option<Vec<(ProjectId, String, SnapshotId)>> {
    _map_to_output_format(&project)
}

pub fn can_map_to_output_format(project: &ItemWithData<Project>) -> bool {
    _map_to_output_format(project).is_some()
}

#[djanco(Dec, 2020, subsets(Generic))]
pub fn sample_stars(database: &Database, _log: &Log, output: &Path) -> Result<(), std::io::Error>  {
    database.projects()
        .filter_by(Equal(project::Language, Language::Java))
        // top stars
        .sort_by(project::Stars)
        .sample(Top(1500))
        // Make sure you don't sample projects that will not convert to output format.
        .filter(can_map_to_output_format)
        // and sample again, this time only valid projects
        .sort_by(project::Stars)
        .sample(Top(1020))
        // Convert to output format (remove projects that failed to convert)
        .flat_map(map_to_output_format)
        // Save to CSV file
        .into_csv_with_headers_in_dir(HEADERS.to_vec(), output, "sample_stars.csv")
}


#[djanco(Dec, 2020, subsets(Generic))]
pub fn sample_all(database: &Database, _log: &Log, output: &Path) -> Result<(), std::io::Error>  {
    database.projects()        
        .filter_by(Equal(project::Language, Language::Java))
        // Make sure you don't sample projects that will not convert to output format.
        .sample(Distinct(Random(SELECTION_SIZE + 1000, Seed(SEED_ALL)), MinRatio(project::Commits, 0.9)))
        .filter(can_map_to_output_format)
        // Just random sample from all projects
        .sample(Distinct(Random(SELECTION_SIZE, Seed(SEED_ALL)), MinRatio(project::Commits, 0.9)))
        // Convert to output format (remove projects that failed to convert)
        .flat_map(map_to_output_format)
        // Save to CSV file
        .into_csv_with_headers_in_dir(HEADERS.to_vec(), output, "sample_all.csv")
}

/* C-Index : 3
   Age : 364.4
   Devs : 3
   Locs : 716.25
   Versions : 20
   Commits : 25.95
   
   
*/
#[djanco(Dec, 2020, subsets(Generic))]
pub fn sample_developed(database: &Database, _log: &Log, output: &Path) -> Result<(), std::io::Error>  {
    database.projects()        
        .filter_by(Equal(project::Language, Language::Java))
        .filter_by(AtLeast(project::MaxHIndex1, 3))
        .filter_by(AtLeast(project::Age, Duration::from_days(364)))
        .filter_by(AtLeast(Count(project::Users), 3))
        .filter_by(AtLeast(project::Locs, 716))
        .filter_by(AtLeast(Count(project::Snapshots), 20))
        .filter_by(AtLeast(Count(project::Commits), 26))
        // Make sure you don't sample proejcts that will not convert to output format.
        .sample(Distinct(Random(SELECTION_SIZE + 1000, Seed(SEED_100LOC_7D_10C)), MinRatio(project::Commits, 0.9)))
        .filter(can_map_to_output_format)
        // Take a random sample 
        .sample(Distinct(Random(SELECTION_SIZE, Seed(SEED_100LOC_7D_10C)), MinRatio(project::Commits, 0.9)))
        // Convert to output format (remove projects that failed to convert)
        .flat_map(map_to_output_format)
        // Save to CSV file
        .into_csv_with_headers_in_dir(HEADERS.to_vec(), output, "sample_100loc_7d_10c.csv")
}

/*
#[djanco(Dec, 2020, subsets(Generic))]
pub fn sample_1000loc_180d_100c(database: &Database, _log: &Log, output: &Path) -> Result<(), std::io::Error>  {
    database.projects()        
        .filter_by(Equal(project::Language, Language::Java))
        // Require projects have at least 100 commits 
        .filter_by(AtLeast(Count(project::Commits), 100))
        // Require projects have at least 180 days of age/lifetime
        .filter_by(AtLeast(project::Age, Duration::from_days(180)))
        // Require projects have at least 1000 LOCs
        .filter_by(AtLeast(project::Locs, 1000))
        // Make sure you don't sample proejcts that will not convert to output format.
        .sample(Distinct(Random(SELECTION_SIZE + 1000, Seed(SEED_1000LOC_180D_100C)), MinRatio(project::Commits, 0.9)))
        .filter(can_map_to_output_format)
        // Take a random sample 
        .sample(Distinct(Random(SELECTION_SIZE, Seed(SEED_1000LOC_180D_100C)), MinRatio(project::Commits, 0.9)))
        // Convert to output format (remove projects that failed to convert)
        .flat_map(map_to_output_format)
        // Save to CSV file
        .into_csv_with_headers_in_dir(HEADERS.to_vec(), output, "sample_1000loc_180d_100c.csv")
}
*/
