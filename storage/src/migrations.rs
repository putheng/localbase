use log::info;
use scylla::Session;

const MIGRATIONS: &[&str] = &[
    // V001 – keyspace
    "CREATE KEYSPACE IF NOT EXISTS storage \
     WITH replication = {'class': 'SimpleStrategy', 'replication_factor': 1} \
     AND durable_writes = true",
    // V002 – buckets
    "CREATE TABLE IF NOT EXISTS storage.buckets ( \
         id uuid, \
         name text, \
         public boolean, \
         created_at timestamp, \
         updated_at timestamp, \
         PRIMARY KEY (id) \
     )",
    // V003a – folders (primary: lookup by bucket + id)
    "CREATE TABLE IF NOT EXISTS storage.folders ( \
         bucket_id uuid, \
         id uuid, \
         parent_id uuid, \
         name text, \
         created_at timestamp, \
         updated_at timestamp, \
         PRIMARY KEY (bucket_id, id) \
     ) WITH CLUSTERING ORDER BY (id ASC)",
    // V003b – folders_by_parent (children listing)
    "CREATE TABLE IF NOT EXISTS storage.folders_by_parent ( \
         bucket_id uuid, \
         parent_id uuid, \
         id uuid, \
         name text, \
         created_at timestamp, \
         updated_at timestamp, \
         PRIMARY KEY ((bucket_id, parent_id), id) \
     ) WITH CLUSTERING ORDER BY (id ASC)",
    // V004a – files (primary: lookup by bucket + id)
    "CREATE TABLE IF NOT EXISTS storage.files ( \
         bucket_id uuid, \
         id uuid, \
         folder_id uuid, \
         name text, \
         size bigint, \
         content_type text, \
         storage_path text, \
         created_at timestamp, \
         updated_at timestamp, \
         PRIMARY KEY (bucket_id, id) \
     ) WITH CLUSTERING ORDER BY (id ASC)",
    // V004b – files_by_folder (folder-scoped listing)
    "CREATE TABLE IF NOT EXISTS storage.files_by_folder ( \
         bucket_id uuid, \
         folder_id uuid, \
         id uuid, \
         name text, \
         size bigint, \
         content_type text, \
         storage_path text, \
         created_at timestamp, \
         updated_at timestamp, \
         PRIMARY KEY ((bucket_id, folder_id), id) \
     ) WITH CLUSTERING ORDER BY (id ASC)",
];

pub async fn run_migrations(session: &Session) -> Result<(), Box<dyn std::error::Error>> {
    for (i, stmt) in MIGRATIONS.iter().enumerate() {
        info!("Running migration {}/{}", i + 1, MIGRATIONS.len());
        session.query_unpaged(*stmt, ()).await?;
    }
    info!("All {} migrations completed", MIGRATIONS.len());
    Ok(())
}
