CREATE TABLE templates (
    template_id UUID NOT NULL PRIMARY KEY,
    create_ts INT NOT NULL
);

CREATE TABLE template_localizations (
    template_id UUID REFERENCES templates (template_id),
    region_id UUID NOT NULL,  -- References db-region.regions
    nomad_job_id STRING NOT NULL,
    PRIMARY KEY (template_id, region_id)
);

