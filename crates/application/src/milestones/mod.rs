pub mod access;
pub mod complete_milestone;
pub mod create_milestone;
pub mod delete_milestone;
pub mod dto;
pub mod get_deal_progress;
pub mod list_milestones;
pub mod start_milestone;
pub mod update_milestone;
pub mod verify_milestone;

pub use complete_milestone::CompleteMilestone;
pub use create_milestone::CreateMilestone;
pub use delete_milestone::DeleteMilestone;
pub use dto::*;
pub use get_deal_progress::GetDealProgress;
pub use list_milestones::ListMilestones;
pub use start_milestone::StartMilestone;
pub use update_milestone::UpdateMilestone;
pub use verify_milestone::VerifyMilestone;

#[cfg(test)]
mod tests;
