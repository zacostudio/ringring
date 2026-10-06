// lucide 아이콘을 kebab-case 이름으로 그린다 — 칸의 아이콘은 이름으로 저장된다
import { icons } from "lucide-react";
import type { LucideProps } from "lucide-react";

interface IconProps extends Omit<LucideProps, "ref"> {
	/** lucide 아이콘 이름 (kebab-case). */
	name: string;
}

/** `chevron-down` → `ChevronDown`. lucide 의 export 이름이다. */
export function toPascalCase(name: string): string {
	return name
		.split("-")
		.map((word) => word.charAt(0).toUpperCase() + word.slice(1))
		.join("");
}

/** 그 이름의 아이콘이 있는가. */
export function iconExists(name: string): boolean {
	return toPascalCase(name) in icons;
}

/**
 * 모르는 이름이면 동그라미를 그린다. 가져온 링의 아이콘 이름이 이 버전의 lucide 에 없을 수 있다 —
 * 칸이 비어 보이면 빈 칸과 구별되지 않는다.
 */
export function Icon({ name, size = 16, ...props }: IconProps) {
	const LucideIcon = icons[toPascalCase(name) as keyof typeof icons] ?? icons.Circle;
	return <LucideIcon size={size} {...props} />;
}
