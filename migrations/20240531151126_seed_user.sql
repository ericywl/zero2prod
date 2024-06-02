-- Add seed user
INSERT INTO users (user_id, username, password_hash)
VALUES (
    'bdd5457a-e361-40eb-b560-dacc801da992',
    'admin',
    '$argon2id$v=19$m=15000,t=2,p=1$UKB3c1MBi+tzILhbZHyXmA$bWqRYWvKUP7ZLvEgnjT0WgD6jKdBEqXdOwrem4S07AI'
);
