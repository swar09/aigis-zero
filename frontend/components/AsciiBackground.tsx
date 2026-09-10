'use client';

import React, { useEffect, useState } from 'react';

const ASCII_ART_1 = `  ______   ______   ______   ______   ______        ________  ________  _______    ______  
 /      \\ /      | /      \\ /      | /      \\      /        |/        |/       \\  /      \\ 
/$$$$$$  |$$$$$$/ /$$$$$$  |$$$$$$/ /$$$$$$  |     $$$$$$$$/ $$$$$$$$/ $$$$$$$  |/$$$$$$  |
$$ |__$$ |  $$ |  $$ | _$$/   $$ |  $$ \\__$$/  ______  /$$/  $$ |__    $$ |__$$ |$$ |  $$ |
$$    $$ |  $$ |  $$ |/    |  $$ |  $$      \\ /      |/$$/   $$    |   $$    $$< $$ |  $$ |
$$$$$$$$ |  $$ |  $$ |$$$$ |  $$ |   $$$$$$  |$$$$$$//$$/    $$$$$/    $$$$$$$  |$$ |  $$ |
$$ |  $$ | _$$ |_ $$ \\__$$ | _$$ |_ /  \\__$$ |      /$$/____ $$ |_____ $$ |  $$ |$$ \\__$$ |
$$ |  $$ |/ $$   |$$    $$/ / $$   |$$    $$/      /$$      |$$       |$$ |  $$ |$$    $$/ 
$$/   $$/ $$$$$$/  $$$$$$/  $$$$$$/  $$$$$$/       $$$$$$$$/ $$$$$$$$/ $$/   $$/  $$$$$$/  `;

const ASCII_ART_2 = `  /$$$$$$  /$$$$$$  /$$$$$$  /$$$$$$  /$$$$$$       /$$$$$$$$ /$$$$$$$$ /$$$$$$$   /$$$$$$ 
 /$$__  $$|_  $$_/ /$$__  $$|_  $$_/ /$$__  $$     |_____ $$ | $$_____/| $$__  $$ /$$__  $$
| $$  \\ $$  | $$  | $$  \\__/  | $$  | $$  \\__/          /$$/ | $$      | $$  \\ $$| $$  \\ $$
| $$$$$$$$  | $$  | $$ /$$$$  | $$  |  $$$$$$  /$$$$$$ /$$/  | $$$$$   | $$$$$$$/| $$  | $$
| $$__  $$  | $$  | $$|_  $$  | $$   \\____  $$|______//$$/   | $$__/   | $$__  $$| $$  | $$
| $$  | $$  | $$  | $$  \\ $$  | $$   /$$  \\ $$       /$$/    | $$      | $$  \\ $$| $$  | $$
| $$  | $$ /$$$$$$|  $$$$$$/ /$$$$$$|  $$$$$$/      /$$$$$$$$| $$$$$$$$| $$  | $$|  $$$$$$/
|__/  |__/|______/ \\______/ |______/ \\______/      |________/|________/|__/  |__/ \\______/ `;

const ASCII_ART_3 = ` $$$$$$\\  $$$$$$\\  $$$$$$\\  $$$$$$\\  $$$$$$\\       $$$$$$$$\\ $$$$$$$$\\ $$$$$$$\\   $$$$$$\\  
$$  __$$\\ \\_$$  _|$$  __$$\\ \\_$$  _|$$  __$$\\      \\____$$  |$$  _____|$$  __$$\\ $$  __$$\\ 
$$ /  $$ |  $$ |  $$ /  \\__|  $$ |  $$ /  \\__|         $$  / $$ |      $$ |  $$ |$$ /  $$ |
$$$$$$$$ |  $$ |  $$ |$$$$\\   $$ |  \\$$$$$$\\ $$$$$$\\  $$  /  $$$$$\\    $$$$$$$  |$$ |  $$ |
$$  __$$ |  $$ |  $$ |\\_$$ |  $$ |   \\____$$\\\\______|$$  /   $$  __|   $$  __$$< $$ |  $$ |
$$ |  $$ |  $$ |  $$ |  $$ |  $$ |  $$\\   $$ |      $$  /    $$ |      $$ |  $$ |$$ |  $$ |
$$ |  $$ |$$$$$$\\ \\$$$$$$  |$$$$$$\\ \\$$$$$$  |     $$$$$$$$\\ $$$$$$$$\\ $$ |  $$ | $$$$$$  |
\\__|  \\__|\\______| \\______/ \\______| \\______/      \\________|\\________|\\__|  \\__| \\______/ `;

const ASCII_ART_4 = `  ______   ______   ______   ______   ______        ________  ________  _______    ______  
 /      \\ /      | /      \\ /      | /      \\      /        |/        |/       \\  /      \\ 
/$$$$$$  |$$$$$$/ /$$$$$$  |$$$$$$/ /$$$$$$  |     $$$$$$$$/ $$$$$$$$/ $$$$$$$  |/$$$$$$  |
$$ |__$$ |  $$ |  $$ | _$$/   $$ |  $$ \\__$$/  ______  /$$/  $$ |__    $$ |__$$ |$$ |  $$ |
$$    $$ |  $$ |  $$ |/    |  $$ |  $$      \\ /      |/$$/   $$    |   $$    $$< $$ |  $$ |
$$$$$$$$ |  $$ |  $$ |$$$$ |  $$ |   $$$$$$  |$$$$$$//$$/    $$$$$/    $$$$$$$  |$$ |  $$ |
$$ |  $$ | _$$ |_ $$ \\__$$ | _$$ |_ /  \\__$$ |      /$$/____ $$ |_____ $$ |  $$ |$$ \\__$$ |
$$ |  $$ |/ $$   |$$    $$/ / $$   |$$    $$/      /$$      |$$       |$$ |  $$ |$$    $$/ 
$$/   $$/ $$$$$$/  $$$$$$/  $$$$$$/  $$$$$$/       $$$$$$$$/ $$$$$$$$/ $$/   $$/  $$$$$$/  `;

const ARTS = [ASCII_ART_1, ASCII_ART_2, ASCII_ART_3, ASCII_ART_4];
const SPEEDS = [58, 46, 68, 50, 62, 44, 56, 48];

interface AsciiBackgroundProps {
  theme?: 'dark' | 'light';
  opacity?: number;
}

export function AsciiBackground({
  theme = 'dark',
  opacity,
}: AsciiBackgroundProps) {
  const [mounted, setMounted] = useState(false);
  const [dimensions, setDimensions] = useState({ width: 1920, height: 1080 });

  useEffect(() => {
    setMounted(true);
    let rAF = 0;
    function handleResize() {
      cancelAnimationFrame(rAF);
      rAF = requestAnimationFrame(() => {
        setDimensions({
          width: window.innerWidth,
          height: window.innerHeight,
        });
      });
    }

    handleResize();
    window.addEventListener('resize', handleResize);
    return () => {
      cancelAnimationFrame(rAF);
      window.removeEventListener('resize', handleResize);
    };
  }, []);

  const defaultOpacity = theme === 'dark' ? 0.18 : 0.04;
  const effectiveOpacity = opacity ?? defaultOpacity;
  const color = theme === 'dark' ? '#ffffff' : '#000000';

  // Compute row count ensuring vertical coverage without black spaces on any zoom
  const rowCount = Math.max(8, Math.ceil(dimensions.height / 105) + 3);
  // Compute repeats per half guaranteeing each half exceeds screen width
  const repeatsPerHalf = Math.max(3, Math.ceil(dimensions.width / 580) + 2);

  if (!mounted) {
    return null;
  }

  return (
    <div
      className="ascii-bg-container"
      aria-hidden="true"
      style={{ opacity: effectiveOpacity }}
    >
      {Array.from({ length: rowCount }).map((_, rowIndex) => {
        const art = ARTS[rowIndex % ARTS.length];
        const isLeft = rowIndex % 2 === 0;
        const duration = SPEEDS[rowIndex % SPEEDS.length];
        const animationClass = isLeft ? 'ascii-scroll-left' : 'ascii-scroll-right';

        return (
          <div key={`row-${rowIndex}`} className="ascii-layer">
            <div
              className={`ascii-track ${animationClass}`}
              style={{ animationDuration: `${duration}s` }}
            >
              <div className="ascii-half">
                {Array.from({ length: repeatsPerHalf }).map((_, i) => (
                  <pre key={`h1-${i}`} style={{ color }}>
                    {art}
                  </pre>
                ))}
              </div>
              <div className="ascii-half" aria-hidden="true">
                {Array.from({ length: repeatsPerHalf }).map((_, i) => (
                  <pre key={`h2-${i}`} style={{ color }}>
                    {art}
                  </pre>
                ))}
              </div>
            </div>
          </div>
        );
      })}
    </div>
  );
}
