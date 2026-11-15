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
  // Compute animation tempo from BPM
  const beatSec = 60 / Math.max(60, Math.min(180, bpm));
  const isHighEnergy =
    ['baraat', 'punjabi_tashan', 'festival_mode', 'desi_club'].includes(moodId) ||
    energy > 0.85;
  const isGarba = ['garba', 'dandiya'].includes(moodId);
  const isAcoustic = ['haldi', 'mehendi'].includes(moodId);

  // Animation timing
  const animDuration = isHighEnergy
    ? `${beatSec * 0.8}s`
    : isGarba
    ? `${beatSec * 1.5}s`
    : `${beatSec}s`;

  return (
    <div className="relative w-full h-full flex items-end justify-center pointer-events-none select-none">
      <style>{`
        @keyframes djBaraatBounce {
          0%, 100% {
            transform: translateY(0px) rotate(0deg) scale(1);
          }
          40% {
            transform: translateY(-20px) rotate(2.2deg) scale(1.025);
          }
          65% {
            transform: translateY(2px) rotate(-0.5deg) scale(0.995);
          }
        }

        @keyframes djGarbaSway {
          0%, 100% {
            transform: translateX(-18px) rotate(-3deg) translateY(0px);
          }
          50% {
            transform: translateX(18px) rotate(3deg) translateY(-10px);
          }
        }

        @keyframes djSmoothGroove {
          0%, 100% {
            transform: translateY(0px) rotate(0deg);
          }
          45% {
            transform: translateY(-11px) rotate(-1.5deg);
          }
          75% {
            transform: translateY(1px) rotate(0.8deg);
          }
        }

        @keyframes djAcousticBob {
          0%, 100% {
            transform: translateY(0px) rotate(-0.5deg);
          }
          50% {
            transform: translateY(-7px) rotate(1.2deg);
          }
        }

        @keyframes djBassPulseGlow {
          0%, 100% {
            filter: drop-shadow(0 15px 30px rgba(0,0,0,0.85)) drop-shadow(0 0 10px rgba(245,158,11,0.2));
          }
          40% {
            filter: drop-shadow(0 25px 50px rgba(0,0,0,0.98)) drop-shadow(0 0 32px rgba(245,158,11,0.75)) drop-shadow(0 0 16px rgba(236,72,153,0.5));
          }
        }

        @keyframes djSpotlightSweep {
          0%, 100% {
            transform: translateX(-30%) rotate(-15deg);
            opacity: 0.25;
          }
          50% {
            transform: translateX(30%) rotate(15deg);
            opacity: 0.55;
          }
        }

        @keyframes djIdleBreathing {
          0%, 100% {
            transform: translateY(0px) scale(1);
          }
          50% {
            transform: translateY(-3px) scale(1.005);
          }
        }

        .dj-character-container {
          transform-origin: bottom center;
          animation: ${
            !isPlaying
              ? 'djIdleBreathing 3.5s infinite ease-in-out'
              : isGarba
              ? `djGarbaSway ${animDuration} infinite ease-in-out, djBassPulseGlow ${beatSec}s infinite ease-in-out`
              : isHighEnergy
              ? `djBaraatBounce ${animDuration} infinite ease-in-out, djBassPulseGlow ${beatSec}s infinite ease-in-out`
              : isAcoustic
              ? `djAcousticBob ${animDuration} infinite ease-in-out, djBassPulseGlow ${beatSec}s infinite ease-in-out`
              : `djSmoothGroove ${animDuration} infinite ease-in-out, djBassPulseGlow ${beatSec}s infinite ease-in-out`
          };
        }
      `}</style>

      {/* High-Resolution Illustrated DJ Character Cutout */}
      <div className="dj-character-container relative w-full h-full flex items-end justify-center">
        {/* Dynamic concert spotlight beam sweeping over DJ */}
        <div
          className={`absolute -top-24 w-[160%] h-[160%] pointer-events-none mix-blend-screen transition-opacity duration-500 blur-2xl ${
            isPlaying ? 'opacity-40' : 'opacity-0'
          }`}
          style={{
            background:
              'radial-gradient(ellipse at 50% 15%, rgba(245,158,11,0.6) 0%, rgba(236,72,153,0.35) 45%, transparent 70%)',
            animation: `djSpotlightSweep ${beatSec * 4}s infinite ease-in-out`,
          }}
        />

        <img
          src="./dj_animated_character.png"
          alt="DHUN Resident DJ"
          className="max-h-[105%] max-w-[95%] object-contain object-bottom select-none pointer-events-none drop-shadow-[0_20px_40px_rgba(0,0,0,0.9)]"
          draggable={false}
        />

        {/* Dynamic party stage lighting highlight reflection on DJ shoulders */}
        <div
          className={`absolute inset-0 pointer-events-none mix-blend-color-dodge transition-opacity duration-300 ${
            isPlaying ? 'opacity-45' : 'opacity-0'
          }`}
          style={{
            background:
              'radial-gradient(circle at 50% 30%, rgba(251,191,36,0.6) 0%, rgba(56,189,248,0.2) 40%, transparent 65%)',
          }}
        />

        {/* Console Deck Dual Turntable Platter Glow under DJ hands */}
        <div className="absolute -bottom-2 left-6 right-6 h-8 flex justify-between pointer-events-none px-4">
          <div
            className={`w-28 h-6 rounded-full bg-cyan-400/25 blur-md border border-cyan-400/50 ${
              isPlaying ? 'animate-[spin_2.5s_linear_infinite]' : ''
            }`}
          />
          <div
            className={`w-28 h-6 rounded-full bg-amber-400/25 blur-md border border-amber-400/50 ${
              isPlaying ? 'animate-[spin_2.5s_linear_infinite]' : ''
            }`}
          />
        </div>
      </div>
    </div>
  );
};
