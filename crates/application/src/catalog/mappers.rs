use crate::catalog::dto::{
    EnhancementPublicResult, EnhancementResult, EnhancementSummaryResult, NeedPublicResult,
    NeedResult, NeedSummaryResult, ResourcePublicResult, ResourceResult, ResourceSummaryResult,
};
use domain::entities::{Enhancement, Need, Resource};

pub fn map_resource_to_result(resource: Resource) -> ResourceResult {
    ResourceResult {
        id: resource.id,
        supplier_party_id: resource.supplier_party_id,
        resource_type_id: resource.resource_type_id,
        resource_name: resource.resource_name,
        description: resource.description,
        quantity: resource.quantity,
        quantity_unit: resource.quantity_unit,
        condition: resource.condition,
        latitude: resource.location.map(|l| l.latitude),
        longitude: resource.location.map(|l| l.longitude),
        location_address: resource.location_address,
        availability_start: resource.availability_start,
        availability_end: resource.availability_end,
        document_urls: resource.document_urls,
        opportunity_cost: resource.opportunity_cost,
        verified_by_platform: resource.verified_by_platform,
        metadata: resource.metadata,
        is_active: resource.is_active,
        deal_count: resource.deal_count,
        platform_hidden: resource.platform_hidden,
        platform_featured: resource.platform_featured,
        admin_notes: resource.admin_notes,
        admin_reviewed_at: resource.admin_reviewed_at,
        admin_reviewed_by: resource.admin_reviewed_by,
        created_at: resource.created_at,
        updated_at: resource.updated_at,
    }
}

pub fn map_resource_to_public(resource: Resource) -> ResourcePublicResult {
    ResourcePublicResult {
        id: resource.id,
        supplier_party_id: resource.supplier_party_id,
        resource_type_id: resource.resource_type_id,
        resource_name: resource.resource_name,
        description: resource.description,
        quantity: resource.quantity,
        quantity_unit: resource.quantity_unit,
        condition: resource.condition,
        availability_start: resource.availability_start,
        availability_end: resource.availability_end,
        document_urls: resource.document_urls,
        verified_by_platform: resource.verified_by_platform,
        metadata: resource.metadata,
        is_active: resource.is_active,
        platform_featured: resource.platform_featured,
        created_at: resource.created_at,
        updated_at: resource.updated_at,
    }
}

pub fn map_resource_to_summary(resource: Resource) -> ResourceSummaryResult {
    ResourceSummaryResult {
        id: resource.id,
        supplier_party_id: resource.supplier_party_id,
        resource_name: resource.resource_name,
        quantity: resource.quantity,
        quantity_unit: resource.quantity_unit,
        condition: resource.condition,
        verified_by_platform: resource.verified_by_platform,
        is_active: resource.is_active,
        platform_featured: resource.platform_featured,
        created_at: resource.created_at,
    }
}

pub fn map_need_to_result(need: Need) -> NeedResult {
    NeedResult {
        id: need.id,
        consumer_party_id: need.consumer_party_id,
        need_category_id: need.need_category_id,
        need_description: need.need_description,
        required_quantity: need.required_quantity,
        quantity_unit: need.quantity_unit,
        quality_requirements: need.quality_requirements,
        required_by_date: need.required_by_date,
        max_budget: need.max_budget,
        budget_currency: need.budget_currency,
        estimated_fulfillment_value: need.estimated_fulfillment_value,
        acceptable_variants: need.acceptable_variants,
        priority: need.priority,
        latitude: need.location.map(|l| l.latitude),
        longitude: need.location.map(|l| l.longitude),
        location_address: need.location_address,
        delivery_preferences: need.delivery_preferences,
        metadata: need.metadata,
        is_active: need.is_active,
        deal_count: need.deal_count,
        platform_hidden: need.platform_hidden,
        platform_featured: need.platform_featured,
        admin_notes: need.admin_notes,
        admin_reviewed_at: need.admin_reviewed_at,
        admin_reviewed_by: need.admin_reviewed_by,
        created_at: need.created_at,
        updated_at: need.updated_at,
    }
}

pub fn map_need_to_public(need: Need) -> NeedPublicResult {
    NeedPublicResult {
        id: need.id,
        consumer_party_id: need.consumer_party_id,
        need_category_id: need.need_category_id,
        need_description: need.need_description,
        required_quantity: need.required_quantity,
        quantity_unit: need.quantity_unit,
        quality_requirements: need.quality_requirements,
        required_by_date: need.required_by_date,
        budget_currency: need.budget_currency,
        acceptable_variants: need.acceptable_variants,
        priority: need.priority,
        delivery_preferences: need.delivery_preferences,
        metadata: need.metadata,
        is_active: need.is_active,
        platform_featured: need.platform_featured,
        created_at: need.created_at,
        updated_at: need.updated_at,
    }
}

pub fn map_need_to_summary(need: Need) -> NeedSummaryResult {
    NeedSummaryResult {
        id: need.id,
        consumer_party_id: need.consumer_party_id,
        need_description: need.need_description,
        required_quantity: need.required_quantity,
        quantity_unit: need.quantity_unit,
        priority: need.priority,
        is_active: need.is_active,
        platform_featured: need.platform_featured,
        created_at: need.created_at,
    }
}

pub fn map_enhancement_to_result(enhancement: Enhancement) -> EnhancementResult {
    EnhancementResult {
        id: enhancement.id,
        enhancer_party_id: enhancement.enhancer_party_id,
        enhancement_type_id: enhancement.enhancement_type_id,
        enhancement_name: enhancement.enhancement_name,
        description: enhancement.description,
        input_quantity: enhancement.input_quantity,
        quantity_unit: enhancement.quantity_unit,
        estimated_input_cost: enhancement.estimated_input_cost,
        service_duration_hours: enhancement.service_duration_hours,
        estimated_completion_days: enhancement.estimated_completion_days,
        deliverables: enhancement.deliverables,
        prerequisites: enhancement.prerequisites,
        skills: enhancement.skills,
        certifications: enhancement.certifications,
        equipment: enhancement.equipment,
        pricing: enhancement.pricing,
        availability: enhancement.availability,
        service_area: enhancement.service_area,
        metadata: enhancement.metadata,
        is_complete: enhancement.is_complete,
        completed_at: enhancement.completed_at,
        is_active: enhancement.is_active,
        deal_count: enhancement.deal_count,
        platform_hidden: enhancement.platform_hidden,
        platform_featured: enhancement.platform_featured,
        admin_notes: enhancement.admin_notes,
        admin_reviewed_at: enhancement.admin_reviewed_at,
        admin_reviewed_by: enhancement.admin_reviewed_by,
        created_at: enhancement.created_at,
        updated_at: enhancement.updated_at,
    }
}

pub fn map_enhancement_to_public(enhancement: Enhancement) -> EnhancementPublicResult {
    EnhancementPublicResult {
        id: enhancement.id,
        enhancer_party_id: enhancement.enhancer_party_id,
        enhancement_type_id: enhancement.enhancement_type_id,
        enhancement_name: enhancement.enhancement_name,
        description: enhancement.description,
        input_quantity: enhancement.input_quantity,
        quantity_unit: enhancement.quantity_unit,
        service_duration_hours: enhancement.service_duration_hours,
        estimated_completion_days: enhancement.estimated_completion_days,
        deliverables: enhancement.deliverables,
        prerequisites: enhancement.prerequisites,
        skills: enhancement.skills,
        certifications: enhancement.certifications,
        equipment: enhancement.equipment,
        availability: enhancement.availability,
        service_area: enhancement.service_area,
        metadata: enhancement.metadata,
        is_complete: enhancement.is_complete,
        is_active: enhancement.is_active,
        platform_featured: enhancement.platform_featured,
        created_at: enhancement.created_at,
        updated_at: enhancement.updated_at,
    }
}

pub fn map_enhancement_to_summary(enhancement: Enhancement) -> EnhancementSummaryResult {
    EnhancementSummaryResult {
        id: enhancement.id,
        enhancer_party_id: enhancement.enhancer_party_id,
        enhancement_name: enhancement.enhancement_name,
        service_duration_hours: enhancement.service_duration_hours,
        estimated_completion_days: enhancement.estimated_completion_days,
        is_active: enhancement.is_active,
        platform_featured: enhancement.platform_featured,
        created_at: enhancement.created_at,
    }
}
