const fs = require('fs'), crypto = require('crypto');
const [src, dst] = process.argv.slice(2);
const flip = crypto.randomInt(2), nonce = crypto.randomBytes(8).toString('hex');
const [one, two] = flip ? ['v3', 'v2'] : ['v2', 'v3'];
fs.copyFileSync(src + '/' + one + '_matched.wav', dst + '/clip_1.wav');
fs.copyFileSync(src + '/' + two + '_matched.wav', dst + '/clip_2.wav');
const key = `clip_1 = ${one}, clip_2 = ${two} (dialogue A/B, seed 1 lens stages 2-5), nonce ${nonce}\n`;
fs.writeFileSync(src + '/key.txt', key);
console.log('sha256 ' + crypto.createHash('sha256').update(key).digest('hex'));
