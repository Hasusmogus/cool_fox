//! Original vector drawing of the DualSense front panel, in its display aspect ratio.

pub const SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="744" height="512" viewBox="0 0 744 512">
  <defs>
    <linearGradient id="shell" x1="0%" y1="0%" x2="84%" y2="100%">
      <stop stop-color="#fafaff"/>
      <stop offset=".27" stop-color="#ffffff"/>
      <stop offset=".64" stop-color="#edebf3"/>
      <stop offset="1" stop-color="#b3b2c0"/>
    </linearGradient>
    <linearGradient id="body" x1="0%" y1="0%" x2="18%" y2="100%">
      <stop stop-color="#181924"/>
      <stop offset=".53" stop-color="#393a47"/>
      <stop offset="1" stop-color="#15151e"/>
    </linearGradient>
    <linearGradient id="touch" x1="0%" y1="0%" x2="70%" y2="100%">
      <stop stop-color="#ffffff"/>
      <stop offset=".69" stop-color="#eeedf6"/>
      <stop offset="1" stop-color="#d9d7e3"/>
    </linearGradient>
    <linearGradient id="key" x1="0%" y1="0%" x2="85%" y2="100%">
      <stop stop-color="#fefeff"/>
      <stop offset=".52" stop-color="#edecf3"/>
      <stop offset="1" stop-color="#b8b8c4"/>
    </linearGradient>
    <linearGradient id="pad" x1="0%" y1="0%" x2="20%" y2="100%">
      <stop stop-color="#f5f4fb"/>
      <stop offset="1" stop-color="#c1c1cd"/>
    </linearGradient>
    <radialGradient id="cap" cx="39%" cy="27%" r="80%">
      <stop stop-color="#44454f"/>
      <stop offset=".74" stop-color="#2e2e38"/>
      <stop offset="1" stop-color="#11121a"/>
    </radialGradient>
    <radialGradient id="rim" cx="45%" cy="37%" r="66%">
      <stop offset=".6" stop-color="#15151c"/>
      <stop offset=".84" stop-color="#30313a"/>
      <stop offset=".94" stop-color="#6d6c7a"/>
      <stop offset="1" stop-color="#15151c"/>
    </radialGradient>
    <g id="leftcover">
      <path fill="url(#shell)" stroke="#33323e" stroke-width="3.2" d="M197 38 C162 23 131 26 102 48 73 70 63 111 51 154 25 246 14 349 17 422 18 465 31 485 53 487 78 489 101 458 137 409 L214 309 C229 287 227 260 213 236 191 198 196 170 205 141 213 113 226 76 224 53Z"/>
      <path fill="none" stroke="#fff" opacity=".82" stroke-width="3.5" d="M26 397 C27 305 47 203 73 126 82 91 102 64 126 52"/>
      <path fill="none" stroke="#b8b8c3" opacity=".74" stroke-width="3" d="M58 476 C94 456 143 375 196 311"/>
    </g>
    <g id="stick">
      <circle r="56" fill="#14141c" stroke="#a6a5b2" stroke-width="2.5"/>
      <circle r="52" fill="url(#rim)"/>
      <circle r="42" fill="url(#cap)" stroke="#11121a" stroke-width="3"/>
      <circle r="37" fill="none" stroke="#5e5e6c" stroke-width="1.6"/>
      <path d="M-30 -20 Q-17 -41 8 -36" fill="none" stroke="#8f8f9d" opacity=".62" stroke-linecap="round" stroke-width="2.4"/>
    </g>
    <g id="dkey">
      <path d="M-11 -37 Q-11 -42 -5 -42 L6 -42 Q12 -42 12 -36 L12 -17 0 -5 -12 -17Z" fill="url(#pad)" stroke="#666672" stroke-width="1.9"/>
      <path d="M-5 -36 H6" fill="none" stroke="#fff" stroke-width="2" stroke-linecap="round"/>
    </g>
    <g id="shoulder">
      <path d="M97 62 Q111 20 145 18 L187 15 Q201 16 208 40 L206 60Z" fill="#373742" stroke="#15151c" stroke-width="4"/>
      <path d="M116 38 Q139 19 184 21" fill="none" stroke="#696875" stroke-width="3"/>
    </g>
  </defs>
  <use xlink:href="#shoulder"/>
  <use xlink:href="#shoulder" transform="translate(744) scale(-1 1)"/>
  <path fill="url(#body)" stroke="#181822" stroke-width="5" stroke-linejoin="round" d="M188 35 C231 31 291 33 372 33 S512 31 556 35 Q623 16 660 77 C695 140 719 257 727 370 735 464 719 490 692 491 665 490 625 431 586 380 L546 335 Q528 320 507 333 Q475 350 438 344 Q373 333 306 344 Q267 350 236 333 Q215 321 199 335 L155 386 C121 432 81 490 51 491 23 489 8 455 17 369 26 254 49 141 84 76 Q122 18 188 35Z"/>
  <use xlink:href="#leftcover"/>
  <use xlink:href="#leftcover" transform="translate(744) scale(-1 1)"/>
  <path d="M193 45 Q221 39 242 44 Q372 29 502 44 Q525 38 551 45 L571 92 Q550 126 534 158 Q518 193 494 202 C452 212 292 212 250 202 Q226 193 210 158Z" fill="url(#shell)"/>
  <path d="M222 54 Q372 31 522 54 C522 90 505 143 483 177 Q476 188 462 190 Q373 199 282 190 Q269 188 261 177 C239 144 224 94 222 54Z" fill="#36364b" stroke="#5a5966" stroke-width="2.5"/>
  <path d="M227 64 Q231 126 266 176 Q272 185 286 187 M517 64 Q513 125 478 176 Q472 185 458 187" fill="none" stroke="#9696fc" stroke-width="6" stroke-linecap="round"/>
  <path d="M227 53 Q372 36 517 53 C515 90 498 140 478 170 Q471 180 457 182 Q372 190 287 182 Q273 180 266 170 C245 139 229 90 227 53Z" fill="url(#touch)" stroke="#ceccd8" stroke-width="2"/>
  <path d="M242 56 Q372 44 499 55" fill="none" stroke="#fff" stroke-width="3.5" stroke-linecap="round"/>
  <g fill="url(#key)" stroke="#73727e" stroke-width="2">
    <rect x="199" y="94" width="12" height="29" rx="5.5" transform="rotate(-17 205 108)"/>
    <rect x="533" y="94" width="12" height="29" rx="5.5" transform="rotate(17 539 108)"/>
  </g>
  <g fill="none" stroke="#62616d" stroke-linecap="round" stroke-width="2">
    <path d="M195 86 l-4 -4 M200 83 l-1 -6 M205 82 l2 -5"/>
    <path d="M536 78h10 M536 82h10 M536 86h10"/>
  </g>
  <g transform="translate(142 157)">
    <circle r="48" fill="#c9c8d1" stroke="#dfdfe6" stroke-width="2"/>
    <circle r="42" fill="#80808d"/>
    <use xlink:href="#dkey"/>
    <use xlink:href="#dkey" transform="rotate(90)"/>
    <use xlink:href="#dkey" transform="rotate(180)"/>
    <use xlink:href="#dkey" transform="rotate(270)"/>
  </g>
  <g transform="translate(600 111)">
    <circle cy="2.6" r="21.8" fill="#858591"/>
    <circle r="21" fill="url(#key)" stroke="#82828f" stroke-width="2"/>
    <path d="M-14 -10 Q-8 -20 4 -17" fill="none" stroke="white" stroke-width="2"/>
    <g stroke="#494955" stroke-width="2.9" fill="none" stroke-linejoin="round" stroke-linecap="round">
      <path d="M0 -10.5 10.5 8.4 H-10.5Z"/>
    </g>
  </g>
  <g transform="translate(641 151)">
    <circle cy="2.6" r="21.8" fill="#858591"/>
    <circle r="21" fill="url(#key)" stroke="#82828f" stroke-width="2"/>
    <path d="M-14 -10 Q-8 -20 4 -17" fill="none" stroke="white" stroke-width="2"/>
    <g stroke="#494955" stroke-width="2.9" fill="none" stroke-linejoin="round" stroke-linecap="round">
      <circle r="10"/>
    </g>
  </g>
  <g transform="translate(559 151)">
    <circle cy="2.6" r="21.8" fill="#858591"/>
    <circle r="21" fill="url(#key)" stroke="#82828f" stroke-width="2"/>
    <path d="M-14 -10 Q-8 -20 4 -17" fill="none" stroke="white" stroke-width="2"/>
    <g stroke="#494955" stroke-width="2.9" fill="none" stroke-linejoin="round" stroke-linecap="round">
      <rect x="-8.8" y="-8.8" width="17.6" height="17.6" rx=".6"/>
    </g>
  </g>
  <g transform="translate(600 192)">
    <circle cy="2.6" r="21.8" fill="#858591"/>
    <circle r="21" fill="url(#key)" stroke="#82828f" stroke-width="2"/>
    <path d="M-14 -10 Q-8 -20 4 -17" fill="none" stroke="white" stroke-width="2"/>
    <g stroke="#494955" stroke-width="2.9" fill="none" stroke-linejoin="round" stroke-linecap="round">
      <path d="M-8 -8 8 8 M8 -8 -8 8"/>
    </g>
  </g>
  <use xlink:href="#stick" transform="translate(266 274)"/>
  <use xlink:href="#stick" transform="translate(478 274)"/>
  <circle cx="358.75" cy="202.00" r="1.24" fill="#a8a7b3"/>
  <circle cx="364.05" cy="202.00" r="1.24" fill="#a8a7b3"/>
  <circle cx="369.35" cy="202.00" r="1.24" fill="#a8a7b3"/>
  <circle cx="374.65" cy="202.00" r="1.24" fill="#a8a7b3"/>
  <circle cx="379.95" cy="202.00" r="1.24" fill="#a8a7b3"/>
  <circle cx="385.25" cy="202.00" r="1.24" fill="#a8a7b3"/>
  <circle cx="353.45" cy="206.80" r="1.24" fill="#a8a7b3"/>
  <circle cx="358.75" cy="206.80" r="1.24" fill="#a8a7b3"/>
  <circle cx="364.05" cy="206.80" r="1.24" fill="#a8a7b3"/>
  <circle cx="369.35" cy="206.80" r="1.24" fill="#a8a7b3"/>
  <circle cx="374.65" cy="206.80" r="1.24" fill="#a8a7b3"/>
  <circle cx="379.95" cy="206.80" r="1.24" fill="#a8a7b3"/>
  <circle cx="385.25" cy="206.80" r="1.24" fill="#a8a7b3"/>
  <circle cx="390.55" cy="206.80" r="1.24" fill="#a8a7b3"/>
  <circle cx="353.45" cy="211.60" r="1.24" fill="#a8a7b3"/>
  <circle cx="358.75" cy="211.60" r="1.24" fill="#a8a7b3"/>
  <circle cx="364.05" cy="211.60" r="1.24" fill="#a8a7b3"/>
  <circle cx="369.35" cy="211.60" r="1.24" fill="#a8a7b3"/>
  <circle cx="374.65" cy="211.60" r="1.24" fill="#a8a7b3"/>
  <circle cx="379.95" cy="211.60" r="1.24" fill="#a8a7b3"/>
  <circle cx="385.25" cy="211.60" r="1.24" fill="#a8a7b3"/>
  <circle cx="390.55" cy="211.60" r="1.24" fill="#a8a7b3"/>
  <circle cx="358.75" cy="216.40" r="1.24" fill="#a8a7b3"/>
  <circle cx="364.05" cy="216.40" r="1.24" fill="#a8a7b3"/>
  <circle cx="369.35" cy="216.40" r="1.24" fill="#a8a7b3"/>
  <circle cx="374.65" cy="216.40" r="1.24" fill="#a8a7b3"/>
  <circle cx="379.95" cy="216.40" r="1.24" fill="#a8a7b3"/>
  <circle cx="385.25" cy="216.40" r="1.24" fill="#a8a7b3"/>
  <circle cx="364.05" cy="221.20" r="1.24" fill="#a8a7b3"/>
  <circle cx="369.35" cy="221.20" r="1.24" fill="#a8a7b3"/>
  <circle cx="374.65" cy="221.20" r="1.24" fill="#a8a7b3"/>
  <circle cx="379.95" cy="221.20" r="1.24" fill="#a8a7b3"/>
  <path d="M362 264 l10 -8 10 8 v10 h-7 v-6 h-6 v6 h-7Z" fill="#6b6b79" stroke="#14151c" stroke-width="1.6" stroke-linejoin="round"/>
  <rect x="359" y="289" width="26" height="8" rx="4" fill="#14151b" stroke="#62616f" stroke-width="1.4"/>
  <path d="M366 293h12" stroke="#aaa9b7" stroke-width="1.3" stroke-linecap="round"/>
  <path d="M334 326 Q371 323 410 326" fill="none" stroke="#545361" stroke-width="2" stroke-linecap="round"/>
</svg>"##;
