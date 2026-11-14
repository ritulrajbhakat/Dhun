import React from 'react';

interface Props {
  bpm: number;
  energy: number;
  moodId: string;
  isPlaying: boolean;
}

export const AnimatedDjCharacter: React.FC<Props> = ({
  bpm,
  energy,
  moodId,
  isPlaying,
}) => {
  // Compute animation speed from BPM
  const beatSec = 60 / Math.max(60, Math.min(180, bpm));
  const isHighEnergy = ['baraat', 'punjabi_tashan', 'festival_mode', 'desi_club'].includes(moodId) || energy > 0.85;
  const isGarba = ['garba', 'dandiya'].includes(moodId);

  // Bounce and sway duration
  const bodyAnimDuration = isHighEnergy ? `${beatSec}s` : `${beatSec * 2}s`;
  const armAnimDuration = isHighEnergy ? `${beatSec * 0.5}s` : `${beatSec}s`;

  return (
    <div className="relative w-full h-full flex items-end justify-center pointer-events-none select-none">
      {/* SVG Animated DJ Rig */}
      <svg
        viewBox="0 0 600 600"
        className="w-[85%] max-w-[580px] h-auto overflow-visible drop-shadow-[0_15px_30px_rgba(0,0,0,0.85)]"
      >
        <defs>
          {/* Skin and clothes gradients */}
          <linearGradient id="skinGrad" x1="0%" y1="0%" x2="100%" y2="100%">
            <stop offset="0%" stopColor="#f6c29b" />
            <stop offset="60%" stopColor="#d89667" />
            <stop offset="100%" stopColor="#b67347" />
          </linearGradient>

          <linearGradient id="shirtGrad" x1="0%" y1="0%" x2="100%" y2="100%">
            <stop offset="0%" stopColor="#f43f5e" />
            <stop offset="35%" stopColor="#f59e0b" />
            <stop offset="70%" stopColor="#06b6d4" />
            <stop offset="100%" stopColor="#a855f7" />
          </linearGradient>

          <linearGradient id="hpGrad" x1="0%" y1="0%" x2="100%" y2="100%">
            <stop offset="0%" stopColor="#334155" />
            <stop offset="50%" stopColor="#0f172a" />
            <stop offset="100%" stopColor="#020617" />
          </linearGradient>

          <filter id="djGlow" x="-20%" y="-20%" width="140%" height="140%">
            <feGaussianBlur stdDeviation="6" result="blur" />
            <feComposite in="SourceGraphic" in2="blur" operator="over" />
          </filter>
        </defs>

        <style>{`
          @keyframes djBounceHigh {
            0%, 100% { transform: translateY(0px) rotate(0deg); }
            50% { transform: translateY(-16px) rotate(1.5deg); }
          }
          @keyframes djBounceSmooth {
            0%, 100% { transform: translateY(0px) rotate(0deg); }
            50% { transform: translateY(-8px) rotate(-1deg); }
          }
          @keyframes djGarbaSway {
            0%, 100% { transform: translateX(-12px) rotate(-2deg); }
            50% { transform: translateX(12px) rotate(2deg); }
          }
          @keyframes djArmPump {
            0%, 100% { transform: rotate(-8deg) translateY(0px); }
            50% { transform: rotate(14deg) translateY(-18px); }
          }
          @keyframes djHeadNod {
            0%, 100% { transform: rotate(0deg) translateY(0px); }
            50% { transform: rotate(3deg) translateY(-6px); }
          }
          @keyframes turntableScratch {
            0%, 100% { transform: rotate(0deg) translateX(0px); }
            50% { transform: rotate(-6deg) translateX(8px); }
          }

          .dj-body-anim {
            transform-origin: 300px 520px;
            animation: ${
              !isPlaying
                ? 'none'
                : isGarba
                ? `djGarbaSway ${bodyAnimDuration} infinite ease-in-out`
                : isHighEnergy
                ? `djBounceHigh ${bodyAnimDuration} infinite ease-in-out`
                : `djBounceSmooth ${bodyAnimDuration} infinite ease-in-out`
            };
          }

          .dj-head-anim {
            transform-origin: 300px 240px;
            animation: ${isPlaying ? `djHeadNod ${bodyAnimDuration} infinite ease-in-out` : 'none'};
          }

          .dj-arm-pump-anim {
            transform-origin: 370px 290px;
            animation: ${isPlaying ? `djArmPump ${armAnimDuration} infinite ease-in-out` : 'none'};
          }

          .dj-hand-mix-anim {
            transform-origin: 220px 320px;
            animation: ${isPlaying ? `turntableScratch ${beatSec}s infinite ease-in-out` : 'none'};
          }
        `}</style>

        {/* DJ Group */}
        <g className="dj-body-anim">
          {/* Torso & Printed Desi Party Shirt */}
          <path
            d="M210 320 C230 300, 370 300, 390 320 L420 540 C420 550, 180 550, 180 540 Z"
            fill="url(#shirtGrad)"
            stroke="#1e293b"
            strokeWidth="3"
          />

          {/* Festive Shirt Tribal/Mandala Pattern Motifs */}
          <g stroke="#ffffff" strokeWidth="2" opacity="0.65" fill="none">
            <path d="M220 360 Q300 400 380 360" />
            <path d="M215 410 Q300 450 385 410" />
            <path d="M210 460 Q300 500 390 460" />
            <circle cx="260" cy="380" r="12" fill="#fbbf24" opacity="0.8" />
            <circle cx="340" cy="380" r="12" fill="#38bdf8" opacity="0.8" />
            <circle cx="300" cy="430" r="15" fill="#f43f5e" opacity="0.8" />
          </g>

          {/* Golden Necklace / Chain */}
          <path
            d="M260 300 Q300 360 340 300"
            fill="none"
            stroke="#f59e0b"
            strokeWidth="4"
            filter="url(#djGlow)"
          />

          {/* Left Arm (Resting on Mixer / Scratching Jog Wheel) */}
          <g className="dj-hand-mix-anim">
            <path
              d="M220 320 Q180 390 200 460 L235 470 Q215 400 245 340 Z"
              fill="url(#skinGrad)"
              stroke="#b67347"
              strokeWidth="2"
            />
            {/* Left Hand on Turntable */}
            <circle cx="215" cy="475" r="18" fill="url(#skinGrad)" />
            <path
              d="M200 465 L225 465 M202 475 L228 475 M205 485 L230 485"
              stroke="#8c4e25"
              strokeWidth="3"
              strokeLinecap="round"
            />
          </g>

          {/* Right Arm (Raised High in the Air Grooving to the Music) */}
          <g className="dj-arm-pump-anim">
            {/* Upper Arm & Forearm */}
            <path
              d="M375 310 C410 270, 420 180, 435 120 L465 130 C450 190, 440 280, 395 330 Z"
              fill="url(#skinGrad)"
              stroke="#b67347"
              strokeWidth="2"
            />
            {/* Raised Hand with Pointing Finger / Party Gesture */}
            <g transform="translate(440, 95)">
              {/* Palm */}
              <circle cx="0" cy="15" r="16" fill="url(#skinGrad)" />
              {/* Pointing Index Finger */}
              <rect x="-4" y="-20" width="8" height="24" rx="4" fill="url(#skinGrad)" />
              {/* Other curled fingers */}
              <rect x="-12" y="2" width="6" height="14" rx="3" fill="url(#skinGrad)" />
              <rect x="6" y="2" width="6" height="14" rx="3" fill="url(#skinGrad)" />
              {/* Wristband / Kada */}
              <rect x="-14" y="28" width="28" height="8" rx="2" fill="#f59e0b" stroke="#78350f" strokeWidth="1" />
            </g>
          </g>

          {/* Neck & Head Group */}
          <g className="dj-head-anim">
            {/* Neck */}
            <rect x="275" y="240" width="50" height="70" rx="10" fill="url(#skinGrad)" />

            {/* Over-Ear Headphones around the Neck */}
            <g>
              {/* Headband curve */}
              <path
                d="M230 265 Q300 325 370 265"
                fill="none"
                stroke="url(#hpGrad)"
                strokeWidth="20"
                strokeLinecap="round"
              />
              {/* Left & Right Cushions */}
              <rect x="220" y="240" width="26" height="38" rx="10" fill="#0f172a" stroke="#f43f5e" strokeWidth="3" />
              <rect x="354" y="240" width="26" height="38" rx="10" fill="#0f172a" stroke="#f43f5e" strokeWidth="3" />
            </g>

            {/* Head Contour */}
            <path
              d="M245 180 C245 110, 355 110, 355 180 C355 240, 245 240, 245 180 Z"
              fill="url(#skinGrad)"
              stroke="#b67347"
              strokeWidth="2"
            />

            {/* Ears */}
            <circle cx="242" cy="180" r="12" fill="url(#skinGrad)" />
            <circle cx="358" cy="180" r="12" fill="url(#skinGrad)" />
            {/* Earring studs */}
            <circle cx="240" cy="188" r="2.5" fill="#fef08a" />
            <circle cx="360" cy="188" r="2.5" fill="#fef08a" />

            {/* Stylish Haircut (Desi Modern Pompadour / Fade) */}
            <path
              d="M235 160 C235 90, 365 75, 365 155 C345 125, 320 120, 300 120 C270 120, 250 135, 235 160 Z"
              fill="#0f172a"
            />
            {/* Hair texture strands */}
            <path
              d="M255 115 Q300 90 345 110"
              stroke="#334155"
              strokeWidth="3"
              fill="none"
            />

            {/* Eyes & Eyebrows (Joyful Party Wink / Big Smile) */}
            <path d="M265 165 Q278 158 290 165" stroke="#0f172a" strokeWidth="3.5" strokeLinecap="round" fill="none" />
            <path d="M310 165 Q322 158 335 165" stroke="#0f172a" strokeWidth="3.5" strokeLinecap="round" fill="none" />

            {/* Smiling Eyes */}
            <path d="M268 175 Q278 168 288 175" stroke="#0f172a" strokeWidth="3" strokeLinecap="round" fill="none" />
            <path d="M312 175 Q322 168 332 175" stroke="#0f172a" strokeWidth="3" strokeLinecap="round" fill="none" />

            {/* Nose */}
            <path d="M300 178 L296 193 Q300 197 304 193 Z" fill="#c27d4c" />

            {/* Big Energetic Happy Smile */}
            <path
              d="M272 202 Q300 230 328 202 Z"
              fill="#ffffff"
              stroke="#991b1b"
              strokeWidth="2.5"
            />
            {/* Tongue / inner mouth */}
            <path d="M285 214 Q300 228 315 214 Z" fill="#ef4444" />

            {/* Festive Tilak / Tika */}
            <rect x="297" y="145" width="6" height="15" rx="3" fill="#dc2626" />
            <circle cx="300" cy="164" r="3" fill="#fbbf24" />
          </g>
        </g>
      </svg>
    </div>
  );
};

