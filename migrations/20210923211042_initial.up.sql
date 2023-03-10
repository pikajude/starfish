create type build_status as enum (
  'queued',
  'building',
  'uploading',
  'succeeded',
  'failed',
  'canceled'
);

create table builds (
  id serial primary key,
  origin varchar(512) not null,
  rev varchar(512) not null,
  created_at timestamptz not null default now(),
  status build_status not null default 'queued',
  finished_at timestamptz null,
  error_msg text null
);

create table inputs (
  id serial primary key,
  build_id integer not null references builds(id),
  path varchar(512) not null
);

create table outputs (
  id serial primary key,
  input_id integer not null references inputs(id),
  system varchar(64) not null,
  drv_path varchar(512) not null,
  store_path varchar(512) not null,
  UNIQUE (input_id, system)
);
