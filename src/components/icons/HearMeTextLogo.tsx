// HearMe wordmark: a listening dot with sound arcs, then the name. Colors come
// from --color-logo-primary so light/dark themes apply exactly like the old logo.
// The brand name is not user-facing copy — it stays identical in every locale.
const BRAND_NAME = "hearme";

const HearMeTextLogo = ({
  width,
  height,
  className,
}: {
  width?: number;
  height?: number;
  className?: string;
}) => {
  return (
    <svg
      width={width}
      height={height}
      className={className}
      viewBox="0 0 1000 300"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
    >
      <circle
        cx={62}
        cy={150}
        r={30}
        style={{ fill: "var(--color-logo-primary)" }}
      />
      <g
        style={{ stroke: "var(--color-logo-primary)" }}
        strokeWidth={26}
        strokeLinecap="round"
        fill="none"
      >
        <path d="M 100 96 A 66 66 0 0 1 100 204" />
        <path d="M 126 58 A 112 112 0 0 1 126 242" />
      </g>
      <text
        x={205}
        y={228}
        style={{ fill: "var(--color-logo-primary)" }}
        fontFamily='"Arial Rounded MT Bold", "SF Pro Rounded", ui-rounded, system-ui, sans-serif'
        fontSize={225}
        fontWeight={700}
        letterSpacing={-6}
      >
        {BRAND_NAME}
      </text>
    </svg>
  );
};

export default HearMeTextLogo;
