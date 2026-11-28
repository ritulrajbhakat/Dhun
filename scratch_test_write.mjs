import fs from 'fs';

const env = fs.readFileSync('c:/Users/ritul/.gemini/antigravity-ide/scratch/dhun/.env', 'utf-8');
const key = env.match(/DHUN_FIREBASE_API_KEY=(.+)/)[1].trim();

const authResp = await fetch('https://identitytoolkit.googleapis.com/v1/accounts:signUp?key=' + key, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ returnSecureToken: true })
});
const authData = await authResp.json();

const raw = JSON.parse(fs.readFileSync('c:/Users/ritul/.gemini/antigravity-ide/scratch/dhun/failed_payload.json', 'utf-8'));
const url = 'https://firestore.googleapis.com/v1/projects/dhun-716e6/databases/(default)/documents/catalogue/fp_71618ddb54338d2e1b9895e2a400999b6dbd4aab6cd6bfb4d61fc2d8b3de7351';

delete raw.fields.double_time_bpm;

// Test with original full beatgrid and full phrases:
const r = await fetch(url, { method: 'PATCH', headers: { 'Content-Type': 'application/json', 'Authorization': 'Bearer ' + authData.idToken }, body: JSON.stringify(raw) });
console.log('Without double_time_bpm (full beatgrid & full phrases): Status', r.status);
if (r.status !== 200) {
    console.log('Error:', await r.text());
}
