const Hypercore = require('hypercore');

// Static test key pair obtained with:
//
//   const crypto = require('hypercore-crypto');
//   const keyPair = crypto.keyPair();
//   console.log("public key", keyPair.publicKey.toString('hex').match(/../g).join(' '));
//   console.log("secret key", keyPair.secretKey.toString('hex').match(/../g).join(' '));
const testKeyPair = {
    publicKey: Buffer.from([
        0x97, 0x60, 0x6c, 0xaa, 0xd2, 0xb0, 0x8c, 0x1d, 0x5f, 0xe1, 0x64, 0x2e, 0xee, 0xa5, 0x62, 0xcb,
        0x91, 0xd6, 0x55, 0xe2, 0x00, 0xc8, 0xd4, 0x3a, 0x32, 0x09, 0x1d, 0x06, 0x4a, 0x33, 0x1e, 0xe3]),
    secretKey: Buffer.from([
        0x27, 0xe6, 0x74, 0x25, 0xc1, 0xff, 0xd1, 0xd9, 0xee, 0x62, 0x5c, 0x96, 0x2b, 0x57, 0x13, 0xc3,
        0x51, 0x0b, 0x71, 0x14, 0x15, 0xf3, 0x31, 0xf6, 0xfa, 0x9e, 0xf2, 0xbf, 0x23, 0x5f, 0x2f, 0xfe,
        0x97, 0x60, 0x6c, 0xaa, 0xd2, 0xb0, 0x8c, 0x1d, 0x5f, 0xe1, 0x64, 0x2e, 0xee, 0xa5, 0x62, 0xcb,
        0x91, 0xd6, 0x55, 0xe2, 0x00, 0xc8, 0xd4, 0x3a, 0x32, 0x09, 0x1d, 0x06, 0x4a, 0x33, 0x1e, 0xe3]),
}

if (process.argv.length !== 4) {
    console.error("Usage: node interop.js [test step] [test set]")
    process.exit(1);
}

if (process.argv[2] === '1') {
    step1(process.argv[3]).then(result => {
        console.log("step1 ready", result);
    });
} else {
    console.error(`Invalid test step {}`, process.argv[2]);
    process.exit(2);
}

async function step1(testSet) {
    let core = new Hypercore(`work/${testSet}`, testKeyPair.publicKey, {keyPair: testKeyPair});
    let len = await core.append(['Hello', 'World', 'foo', 'bar']);
    console.log("LEN", len);
    len = await core.append(['zan', 'tup']);
    console.log("LEN2", len);
    len = await core.append(['fup']);
    console.log("LEN3", len);
    len = await core.append(['sup']);
    console.log("LEN4", len);
    // len = await core.append(['flushes']);
    // console.log("LEN5", len);
    await core.close();
    core = new Hypercore(`work/${testSet}`, testKeyPair.publicKey, {keyPair: testKeyPair});
};
