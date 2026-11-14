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
          50% {
            transform: translateY(-16px) rotate(1.8deg) scale(1.02);
          }
        }

        @keyframes djGarbaSway {
          0%, 100% {
            transform: translateX(-16px) rotate(-2.5deg) translateY(0px);
          }
          50% {
            transform: translateX(16px) rotate(2.5deg) translateY(-8px);
          }
        }

        @keyframes djSmoothGroove {
          0%, 100% {
            transform: translateY(0px) rotate(0deg);
          }
          50% {
            transform: translateY(-9px) rotate(-1.2deg);
          }
        }

        @keyframes djAcousticBob {
          0%, 100% {
            transform: translateY(0px) rotate(-0.5deg);
          }
          50% {
            transform: translateY(-6px) rotate(1deg);
          }
        }

        @keyframes djBassPulseGlow {
          0%, 100% {
            filter: drop-shadow(0 15px 30px rgba(0,0,0,0.85)) drop-shadow(0 0 10px rgba(245,158,11,0.2));
          }
          50% {
            filter: drop-shadow(0 20px 45px rgba(0,0,0,0.95)) drop-shadow(0 0 28px rgba(245,158,11,0.65));
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
        <img
          src="./dj_animated_character.png"
          alt="DHUN Resident DJ"
          className="max-h-[105%] max-w-[95%] object-contain object-bottom select-none pointer-events-none drop-shadow-[0_20px_40px_rgba(0,0,0,0.9)]"
          draggable={false}
        />

        {/* Dynamic party stage lighting highlight reflection on DJ shoulders */}
        <div
          className={`absolute inset-0 pointer-events-none mix-blend-color-dodge transition-opacity duration-300 ${
            isPlaying ? 'opacity-35' : 'opacity-0'
          }`}
          style={{
            background:
              'radial-gradient(circle at 50% 30%, rgba(251,191,36,0.5) 0%, transparent 60%)',
          }}
        />
      </div>
    </div>
  );
};
