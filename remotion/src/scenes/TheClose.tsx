import React from "react";
import {
    AbsoluteFill,
    interpolate,
    spring,
    useCurrentFrame,
    useVideoConfig,
} from "remotion";
import { COLORS, FONT_SIZES } from "../utils/colors";
import { FONTS } from "../utils/fonts";

export const TheClose: React.FC = () => {
    const frame = useCurrentFrame();
    const { fps } = useVideoConfig();

    // Phase 1: Logo appears (0 - 1s)
    const logoEntrance = spring({
        frame,
        fps,
        config: { damping: 200 },
        durationInFrames: fps * 0.8,
    });

    // Glow pulse
    const glowPulse = interpolate(
        frame % (fps * 2),
        [0, fps, fps * 2],
        [0.3, 0.8, 0.3],
        { extrapolateRight: "clamp" }
    );

    // Phase 2: Tagline (1.5s)
    const taglineEntrance = spring({
        frame: Math.max(0, frame - fps * 1.5),
        fps,
        config: { damping: 200 },
        durationInFrames: fps * 0.6,
    });

    // Phase 3: Links (3s)
    const linksEntrance = spring({
        frame: Math.max(0, frame - fps * 3),
        fps,
        config: { damping: 200 },
        durationInFrames: fps * 0.6,
    });

    // Phase 4: Open source badge (4s)
    const badgeEntrance = spring({
        frame: Math.max(0, frame - fps * 4),
        fps,
        config: { damping: 200 },
        durationInFrames: fps * 0.6,
    });

    return (
        <AbsoluteFill
            style={{
                backgroundColor: COLORS.DARKER_BG,
                justifyContent: "center",
                alignItems: "center",
            }}
        >
            {/* Large radial glow */}
            <div
                style={{
                    position: "absolute",
                    width: 1000,
                    height: 1000,
                    borderRadius: "50%",
                    background: `radial-gradient(circle, rgba(139,92,246,${glowPulse * 0.15}) 0%, transparent 60%)`,
                }}
            />

            {/* Logo */}
            <div
                style={{
                    opacity: logoEntrance,
                    transform: `scale(${interpolate(logoEntrance, [0, 1], [0.7, 1])})`,
                    display: "flex",
                    flexDirection: "column",
                    alignItems: "center",
                    gap: 20,
                }}
            >
                {/* Shield */}
                <svg
                    width={100}
                    height={100}
                    viewBox="0 0 24 24"
                    fill="none"
                    style={{
                        filter: `drop-shadow(0 0 ${interpolate(glowPulse, [0, 1], [10, 40])}px ${COLORS.PURPLE})`,
                    }}
                >
                    <path
                        d="M12 2 L3 7 L3 13 C3 18 7 22 12 23 C17 22 21 18 21 13 L21 7 Z"
                        fill={COLORS.PURPLE}
                        opacity={0.3}
                        stroke={COLORS.PURPLE}
                        strokeWidth={1.5}
                    />
                    <path
                        d="M9 12 L11 14 L15 10"
                        stroke={COLORS.WHITE}
                        strokeWidth={2}
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        fill="none"
                    />
                </svg>

                {/* Title */}
                <div
                    style={{
                        fontFamily: FONTS.INTER,
                        fontSize: FONT_SIZES.HERO,
                        fontWeight: 800,
                        color: COLORS.WHITE,
                        letterSpacing: 4,
                    }}
                >
                    Govrix Scout
                </div>
            </div>

            {/* Tagline */}
            <div
                style={{
                    position: "absolute",
                    top: "60%",
                    opacity: taglineEntrance,
                    transform: `translateY(${interpolate(taglineEntrance, [0, 1], [20, 0])}px)`,
                }}
            >
                <div
                    style={{
                        fontFamily: FONTS.INTER,
                        fontSize: FONT_SIZES.BODY,
                        fontWeight: 400,
                        color: COLORS.WHITE_DIM,
                        textAlign: "center",
                    }}
                >
                    Know what your AI agents are doing — before your auditor asks.
                </div>
            </div>

            {/* Links */}
            <div
                style={{
                    position: "absolute",
                    top: "72%",
                    display: "flex",
                    flexDirection: "column",
                    alignItems: "center",
                    gap: 16,
                    opacity: linksEntrance,
                    transform: `translateY(${interpolate(linksEntrance, [0, 1], [20, 0])}px)`,
                }}
            >
                <div
                    style={{
                        fontFamily: FONTS.MONO,
                        fontSize: FONT_SIZES.BODY,
                        color: COLORS.PURPLE,
                        fontWeight: 700,
                    }}
                >
                    ⭐ github.com/Govrix-AI/govrix-scout
                </div>
                <div
                    style={{
                        fontFamily: FONTS.INTER,
                        fontSize: FONT_SIZES.CAPTION,
                        color: COLORS.WHITE_DIM,
                    }}
                >
                    🌐 govrix.dev
                </div>
            </div>

            {/* Open source badge */}
            <div
                style={{
                    position: "absolute",
                    bottom: 60,
                    opacity: badgeEntrance,
                    display: "flex",
                    gap: 24,
                    alignItems: "center",
                }}
            >
                {["🛡️ Apache 2.0", "100% Open Source", "Self-Hosted", "Your Data Never Leaves"].map(
                    (text, i) => (
                        <React.Fragment key={i}>
                            {i > 0 && (
                                <span style={{ color: COLORS.WHITE_DIM, fontSize: 14 }}>·</span>
                            )}
                            <span
                                style={{
                                    fontFamily: FONTS.INTER,
                                    fontSize: 15,
                                    color: COLORS.WHITE_DIM,
                                    fontWeight: 500,
                                }}
                            >
                                {text}
                            </span>
                        </React.Fragment>
                    )
                )}
            </div>
        </AbsoluteFill>
    );
};
