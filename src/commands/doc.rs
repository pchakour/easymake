use crate::doc::{
    action::{ActionDocEntry, TypeDocEntry},
    secret::SecretDocEntry,
};
use lazy_static::lazy_static;
use minijinja::render;
use std::{env, fs, path::PathBuf};

lazy_static! {
    static ref SPECIAL_TYPES: Vec<(&'static str, &'static str)> =
        vec![("InFile", "../types.md#infile",)];
}

pub fn generate() {
    let repository_path = env::current_dir().unwrap();
    let documentation_folder_path = repository_path.join("documentation/src/content/docs/reference");

    // Generate actions documentation
    generate_actions_doc(&documentation_folder_path);

    // Generate types documentation
    // generate_types_doc(&documentation_folder_path);

    // Generate secrets documentation
    generate_secret_doc(&documentation_folder_path);
}

pub fn generate_secret_doc(documentation_folder_path: &PathBuf) {
    // let secrets_doc_path = documentation_folder_path.join("secrets.md");
    let secrets_doc_folder = documentation_folder_path.join("secrets");

    // if secrets_doc_path.exists() {
    //     fs::remove_file(&secrets_doc_path).unwrap();
    // }

    if secrets_doc_folder.exists() {
        fs::remove_dir_all(&secrets_doc_folder).unwrap();
    }

    fs::create_dir(&secrets_doc_folder).unwrap();

    for doc in inventory::iter::<SecretDocEntry> {
        println!("Action {}", doc.id);
        let mut secret_doc = String::from("---\n");
        secret_doc.push_str(&format!("title: {}\n", doc.id));
        secret_doc.push_str(&format!("description: {}\n", doc.short_desc));
        secret_doc.push_str("---\n");
        secret_doc.push_str(&format!("{}\n\n{}\n\n", doc.short_desc, doc.description));
        secret_doc.push_str(&format!("## Example:\n```yaml\n{}\n```\n", doc.example));
        fs::write(&secrets_doc_folder.join(format!("{}.md", doc.id)), secret_doc).unwrap();
    }

    // fs::write(&secrets_doc_path, secrets_documentation).unwrap();
}

/**
 * Generate types documentation
 */
pub fn generate_types_doc(documentation_folder_path: &PathBuf) {
    let types_doc_path = documentation_folder_path.join("types.md");

    if types_doc_path.exists() {
        fs::remove_file(&types_doc_path).unwrap();
    }

    let mut types_documentation = String::from("# Easymake\n\n## Types\n\n");
    let mut types_summary = String::from("| Name | Description |\n");
    types_summary.push_str("| ---- | ---------- |\n");

    let mut types_details = String::from("");

    // Iterate over available types
    for doc in inventory::iter::<TypeDocEntry> {
        types_summary.push_str(&format!("| {} | {} |\n", doc.name, doc.short_desc));
        types_details.push_str(&format!("### {}\n\n", doc.name));
        types_details.push_str(&format!("{}\n\n", doc.short_desc));
        types_details.push_str(&format!("{}\n\n", doc.description));
    }

    types_documentation.push_str(&types_summary);
    types_documentation.push_str("\n\n");
    types_documentation.push_str(&types_details);

    fs::write(&types_doc_path, types_documentation).unwrap();
}

/**
 * Generate actions documentation
 */
pub fn generate_actions_doc(documentation_folder_path: &PathBuf) {
    // Remove existing documentation
    let actions_doc_folder_path = documentation_folder_path.join("actions");

    if actions_doc_folder_path.exists() {
        fs::remove_dir_all(&actions_doc_folder_path).unwrap();
    }

    // Create actions documentation folder
    fs::create_dir(&actions_doc_folder_path).unwrap();

    // let mut actions_summary = String::from("| Name | Description |\n");
    // actions_summary.push_str("| ---- | ---------- |\n");

    // Iterate over available actions
    for doc in inventory::iter::<ActionDocEntry> {
        println!("Action {}", doc.id);
        // actions_summary.push_str(&format!(
        //     "| [{}](./actions/{}.md) | {} |\n",
        //     doc.id, doc.id, doc.short_desc
        // ));

        // Prepare the file content
        let mut content = String::from("---\n");
        content.push_str(&format!("title: {}\n", doc.id));
        content.push_str(&format!("description: {}\n", doc.short_desc));
        content.push_str("---\n");
        content.push_str(&format!("{}\n{}\n\n", doc.short_desc, doc.description));
        content.push_str(&format!("## Example\n\n```yaml\n{}\n```\n\n", doc.example));
        content.push_str(&format!("## Configuration options\n\n"));
        content.push_str("| Name | Description | Type | Required |\n");
        content.push_str(&format!("| ---- | ----------- | -- | -- |\n"));

        for property in doc.properties {
            let mut ty = String::from(property.ty);

            for (special_type, link) in SPECIAL_TYPES.iter() {
                ty = ty.replace(special_type, &format!("[{special_type}]({link})"));
            }
            content.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                property.name, property.description, ty, property.required
            ));
        }

        // Write content in file
        fs::write(
            actions_doc_folder_path.join(format!("{}.md", doc.id)),
            content,
        )
        .unwrap();
    }

    // Generate actions
    // let actions_template_file_path =
    //     documentation_folder_path.join("assets/templates/actions.md.jinja");
    // let actions_template_file_content = fs::read(actions_template_file_path).unwrap();
    // let actions_template_file_content_str =
    //     std::str::from_utf8(&actions_template_file_content).unwrap();
    // let actions_file_content =
    //     render!(actions_template_file_content_str, actions => actions_summary);

    // let actions_file_path = documentation_folder_path.join("actions.md");
    // if actions_file_path.exists() {
    //     fs::remove_file(&actions_file_path).unwrap();
    // }

    // fs::write(actions_file_path, &actions_file_content).unwrap();
}
