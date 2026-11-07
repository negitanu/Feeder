## Purpose

Safely proxy optional article artwork without changing reading state.

## Requirements

### Requirement: Optional same-origin article hero
Detail and visit responses SHALL include a nullable image_url pointing to an authenticated same-origin endpoint when the stored original URL is eligible. A client SHALL tolerate unavailable images.

#### Scenario: Image available
- **WHEN** an authenticated client requests the candidate endpoint
- **THEN** the server may return a bounded verified raster image discovered in the stored article's page metadata

#### Scenario: Image unavailable or unsafe
- **WHEN** metadata, page, image, or any redirect is unavailable or unsafe
- **THEN** the endpoint returns an error without returning remote HTML or SVG

#### Scenario: Data preservation
- **WHEN** the feature is deployed
- **THEN** it does not modify the schema, saved data, read state, or visits
