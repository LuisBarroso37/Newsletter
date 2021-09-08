-- Create 'users' table
CREATE TABLE users(
    id uuid NOT NULL PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    password TEXT NOT NULL
);
