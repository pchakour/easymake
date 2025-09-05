use crate::doc::action::ActionDocEntry;
use std::{env, fs};
use minijinja::render;

pub fn generate() {
  let repository_path = env::current_dir().unwrap();
  let documentation_folder_path = repository_path.join("documentation");

  // Generate actions documentation

  // Remove existing documentation
  let actions_doc_folder_path = documentation_folder_path.join("actions");

  if actions_doc_folder_path.exists() {
    fs::remove_dir_all(&actions_doc_folder_path).unwrap();
  }

  // Create actions documentation folder
  fs::create_dir(&actions_doc_folder_path).unwrap();

  let mut actions_summary = String::from("| Name | Description |\n");
  actions_summary.push_str("| ---- | ---------- |\n");

  // Iterate over available actions
  for doc in inventory::iter::<ActionDocEntry> {
      actions_summary.push_str(&format!("| [{}](./actions/{}.md) | {} |\n", doc.id, doc.id, doc.short_desc));

      // Prepare the file content
      let mut content = String::from("");
      content.push_str(&format!("# Action: {}\n\n", doc.id));
      content.push_str(&format!("## Description\n\n{}\n{}\n\n", doc.short_desc, doc.description));
      content.push_str(&format!("## Example\n\n```yaml\n{}\n```\n\n", doc.example));
      content.push_str(&format!("## Configuration options\n\n"));
      content.push_str("| Name | Description | Type | Required |\n");
      content.push_str(&format!("| ---- | ----------- | -- | -- |\n"));

      for property in doc.properties {
          content.push_str(&format!(
              "| {} | {} | {} | {} |\n",
              property.name, property.description, property.ty, property.required
          ));
      }

      // Write content in file
      fs::write(
          actions_doc_folder_path.join(format!("{}.md", doc.id)),
          content,
      ).unwrap();
  }

  // Generate targets
  let targets_template_file_path = documentation_folder_path.join("assets/templates/targets.md.jinja");
  let targets_template_file_content = fs::read(targets_template_file_path).unwrap();
  let targets_template_file_content_str = std::str::from_utf8(&targets_template_file_content).unwrap();
  let targets_file_content = render!(targets_template_file_content_str, actions => actions_summary);

  let targets_file_path = documentation_folder_path.join("targets.md");
  if targets_file_path.exists() {
    fs::remove_file(&targets_file_path).unwrap();
  }

  fs::write(targets_file_path, &targets_file_content).unwrap();
}
