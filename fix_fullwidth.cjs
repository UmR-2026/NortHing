const fs = require('fs');
const files = [
    'src/crates/assembly/core/src/agentic/execution/write_content_sanitizer.rs',
    'src/crates/assembly/core/src/agentic/insights/html.rs',
    'src/crates/assembly/core/src/service/i18n/model_copy.rs',
    'src/crates/assembly/core/src/service/remote_connect/bot/feishu.rs',
    'src/crates/assembly/core/src/service/remote_connect/bot/locale.rs',
    'src/crates/assembly/core/src/service/remote_connect/bot/telegram.rs',
    'src/crates/assembly/core/src/service/remote_connect/bot/weixin.rs',
    'src/crates/assembly/core/src/util/json_extract.rs'
];

files.forEach(f => {
    const buf = fs.readFileSync(f);
    let changed = false;
    const newBuf = Buffer.alloc(buf.length);
    for(let i = 0; i < buf.length; i++) {
        const b = buf[i];
        // Replace full-width punctuation (0xEF 0xBC 0x8x -> ASCII equivalent)
        if(b === 0xef && buf[i+1] === 0xbc) {
            if(buf[i+2] === 0x8c) { newBuf[i] = 0x2c; i += 2; changed = true; continue; } // ，
            if(buf[i+2] === 0x8e) { newBuf[i] = 0x2e; i += 2; changed = true; continue; } // 。
            if(buf[i+2] === 0x8d) { newBuf[i] = 0x3f; i += 2; changed = true; continue; } // ？
            if(buf[i+2] === 0x81) { newBuf[i] = 0x21; i += 2; changed = true; continue; } // ！
            if(buf[i+2] === 0x88) { newBuf[i] = 0x28; i += 2; changed = true; continue; } // （
            if(buf[i+2] === 0x89) { newBuf[i] = 0x29; i += 2; changed = true; continue; } // ）
        }
        // Replace curly quotes
        if(b === 0xe2 && buf[i+1] === 0x80 && buf[i+2] === 0x98) { newBuf[i] = 0x27; i += 2; changed = true; continue; } // '
        if(b === 0xe2 && buf[i+1] === 0x80 && buf[i+2] === 0x99) { newBuf[i] = 0x27; i += 2; changed = true; continue; } // '
        if(b === 0xe2 && buf[i+1] === 0x80 && buf[i+2] === 0x9c) { newBuf[i] = 0x22; i += 2; changed = true; continue; } // "
        if(b === 0xe2 && buf[i+1] === 0x80 && buf[i+2] === 0x9d) { newBuf[i] = 0x22; i += 2; changed = true; continue; } // "
        newBuf[i] = b;
    }
    if(changed) {
        fs.writeFileSync(f, newBuf);
        console.log('Fixed:', f);
    }
});
console.log('Done');