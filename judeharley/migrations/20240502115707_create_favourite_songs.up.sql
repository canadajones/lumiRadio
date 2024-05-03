CREATE TABLE favourite_songs (
    id SERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id),
    song_id VARCHAR(64) NOT NULL
);