use std::path::Path;

use anyhow::{Context, Result};
use git2::{BranchType, Repository};

pub struct GitOps {
   repo: Repository,
}

impl GitOps {
   pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
      let repo = Repository::discover(path).context("Not a git repository")?;
      Ok(Self { repo })
   }

   pub fn create_branch(&self, branch_name: &str) -> Result<String> {
      // Get current HEAD commit
      let head = self.repo.head().context("Failed to get HEAD")?;
      let commit = head
         .peel_to_commit()
         .context("Failed to resolve HEAD to commit")?;

      // Check if branch already exists
      if self
         .repo
         .find_branch(branch_name, BranchType::Local)
         .is_ok()
      {
         anyhow::bail!("Branch '{}' already exists", branch_name);
      }

      // Create new branch
      self
         .repo
         .branch(branch_name, &commit, false)
         .context("Failed to create branch")?;

      // Switch to new branch
      self
         .repo
         .set_head(&format!("refs/heads/{}", branch_name))
         .context("Failed to switch to new branch")?;

      // Update working directory
      self
         .repo
         .checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
         .context("Failed to checkout new branch")?;

      Ok(branch_name.to_string())
   }

   pub fn current_branch(&self) -> Result<String> {
      let head = self.repo.head().context("Failed to get HEAD")?;
      let branch_name = head
         .shorthand()
         .ok_or_else(|| anyhow::anyhow!("HEAD is detached"))?;
      Ok(branch_name.to_string())
   }

   pub fn create_commit(&self, message: &str) -> Result<String> {
      let mut index = self.repo.index().context("Failed to get index")?;

      // Check if there are changes to commit
      if index.is_empty() {
         anyhow::bail!("No changes to commit");
      }

      // Write the tree
      let tree_id = index.write_tree().context("Failed to write tree")?;
      let tree = self
         .repo
         .find_tree(tree_id)
         .context("Failed to find tree")?;

      // Get parent commit
      let parent_commit = self.repo.head()?.peel_to_commit()?;

      // Get signature (use repo config or defaults)
      let sig = self
         .repo
         .signature()
         .context("Failed to get git signature. Configure git user.name and user.email")?;

      // Create commit
      let commit_id = self
         .repo
         .commit(Some("HEAD"), &sig, &sig, message, &tree, &[&parent_commit])
         .context("Failed to create commit")?;

      Ok(commit_id.to_string())
   }

   pub fn has_staged_changes(&self) -> Result<bool> {
      let statuses = self.repo.statuses(None)?;

      for entry in statuses.iter() {
         let status = entry.status();
         if status.contains(git2::Status::INDEX_NEW)
            || status.contains(git2::Status::INDEX_MODIFIED)
            || status.contains(git2::Status::INDEX_DELETED)
            || status.contains(git2::Status::INDEX_RENAMED)
            || status.contains(git2::Status::INDEX_TYPECHANGE)
         {
            return Ok(true);
         }
      }

      Ok(false)
   }
}
