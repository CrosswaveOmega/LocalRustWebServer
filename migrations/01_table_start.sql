-- Create users table.
create table if not exists users
(
    id integer primary key not null,
    username text not null unique,
    password text not null,
    access_level integer not null
);
