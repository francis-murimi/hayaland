-- Seed initial category taxonomy.
-- Idempotent: uses ON CONFLICT on category_code.

INSERT INTO categories (id, category_code, category_name, category_type, description, display_order)
VALUES
    ('a1b2c3d4-e5f6-7890-abcd-ef1234567890', 'agriculture', 'Agriculture', 'DOMAIN', 'Farming, crops, livestock, and agricultural resources', 1),
    ('b2c3d4e5-f6a7-8901-bcde-f12345678901', 'real-estate', 'Real Estate', 'DOMAIN', 'Land, buildings, rental spaces, and property', 2),
    ('c3d4e5f6-a7b8-9012-cdef-123456789012', 'transportation', 'Transportation', 'DOMAIN', 'Vehicles, logistics, and transport capacity', 3),
    ('d4e5f6a7-b8c9-0123-defa-234567890123', 'manufacturing', 'Manufacturing', 'DOMAIN', 'Machinery, materials, and production capacity', 4),
    ('e5f6a7b8-c9d0-1234-efab-345678901234', 'technology', 'Technology', 'DOMAIN', 'Data, software, hardware, and technology services', 5)
ON CONFLICT (category_code) DO UPDATE SET
    category_name = EXCLUDED.category_name,
    category_type = EXCLUDED.category_type,
    description = EXCLUDED.description,
    display_order = EXCLUDED.display_order,
    updated_at = now();

INSERT INTO categories (id, parent_category_id, category_code, category_name, category_type, description, display_order)
VALUES
    ('f6a7b8c9-d0e1-2345-fabc-456789012345', 'a1b2c3d4-e5f6-7890-abcd-ef1234567890', 'farmland', 'Farmland', 'RESOURCE_TYPE', 'Arable land and farm plots', 1),
    ('a7b8c9d0-e1f2-3456-abcd-567890123456', 'a1b2c3d4-e5f6-7890-abcd-ef1234567890', 'crop-produce', 'Crop Produce', 'NEED_TYPE', 'Fruits, vegetables, grains, and other crops', 2),
    ('b8c9d0e1-f2a3-4567-bcde-678901234567', 'a1b2c3d4-e5f6-7890-abcd-ef1234567890', 'agro-inputs', 'Agro Inputs', 'ENHANCEMENT_TYPE', 'Seeds, fertilizer, and agronomic expertise', 3),
    ('c9d0e1f2-a3b4-5678-cdef-789012345678', 'b2c3d4e5-f6a7-8901-bcde-f12345678901', 'vacant-building', 'Vacant Building', 'RESOURCE_TYPE', 'Unused buildings and commercial space', 4),
    ('d0e1f2a3-b4c5-6789-defa-890123456789', 'b2c3d4e5-f6a7-8901-bcde-f12345678901', 'rental-space', 'Rental Space', 'NEED_TYPE', 'Short or long-term rental space', 5),
    ('e1f2a3b4-c5d6-7890-efab-901234567890', 'b2c3d4e5-f6a7-8901-bcde-f12345678901', 'renovation', 'Renovation', 'ENHANCEMENT_TYPE', 'Contracting and renovation services', 6),
    ('f2a3b4c5-d6e7-8901-fabc-012345678901', 'c3d4e5f6-a7b8-9012-cdef-123456789012', 'vehicle-capacity', 'Vehicle Capacity', 'RESOURCE_TYPE', 'Idle vehicles and fleet capacity', 7),
    ('a3b4c5d6-e7f8-9012-abcd-123456789012', 'c3d4e5f6-a7b8-9012-cdef-123456789012', 'transport-service', 'Transport Service', 'NEED_TYPE', 'Transportation and logistics services', 8),
    ('b4c5d6e7-f8a9-0123-bcde-234567890123', 'c3d4e5f6-a7b8-9012-cdef-123456789012', 'fleet-maintenance', 'Fleet Maintenance', 'ENHANCEMENT_TYPE', 'Vehicle maintenance and repair', 9)
ON CONFLICT (category_code) DO UPDATE SET
    parent_category_id = EXCLUDED.parent_category_id,
    category_name = EXCLUDED.category_name,
    category_type = EXCLUDED.category_type,
    description = EXCLUDED.description,
    display_order = EXCLUDED.display_order,
    updated_at = now();
