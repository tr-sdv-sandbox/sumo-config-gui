import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

type JsonMap = Record<string, unknown>;

interface Deployment {
	channel: string;
	profile: string;
}

interface PartConfig {
	id: string;
	kind: string;
	source?: string | null;
}

interface ComponentConfig {
	path: string;
	kind: string;
	version?: string | null;
	update_mode?: string | null;
	target: JsonMap;
	parts: PartConfig[];
}

interface VehicleConfig {
	key: string;
	id: string;
	kind: string;
	deployment: Deployment;
	target: JsonMap;
	labels: JsonMap;
	components: ComponentConfig[];
	disabled: boolean;
	schema: string;
	source_path: string;
}

interface LinkStatus {
	available: boolean;
	state: string;
	message?: string | null;
}

interface TowerLinkage {
	tower1_device: LinkStatus;
	tower2_channel: LinkStatus;
}

interface VehicleSummary {
	key: string;
	id: string;
	kind: string;
	channel: string;
	profile: string;
	schema: string;
	source_path: string;
	disabled: boolean;
	component_count: number;
	part_count: number;
	linkage?: TowerLinkage | null;
}

interface ValidationResult {
	valid: boolean;
	errors: string[];
	warnings: string[];
}

interface CommandResponse<T> {
	value: T;
	validation: ValidationResult;
}

interface CloneOptions {
	new_id: string;
	channel?: string | null;
	profile?: string | null;
}

interface CloneDialogState {
	source: VehicleSummary;
	newId: string;
	channel: string;
	profile: string;
}

const DEFAULT_TOWER1 = "http://localhost:8080";
const DEFAULT_TOWER2 = "http://localhost:8081";

export default function App() {
	const [root, setRoot] = useState("");
	const [tower1Url, setTower1Url] = useState(DEFAULT_TOWER1);
	const [tower2Url, setTower2Url] = useState(DEFAULT_TOWER2);
	const [vehicles, setVehicles] = useState<VehicleSummary[]>([]);
	const [expandedKey, setExpandedKey] = useState<string | null>(null);
	const [selectedVehicle, setSelectedVehicle] = useState<VehicleConfig | null>(
		null,
	);
	const [editorText, setEditorText] = useState("");
	const [editorOpen, setEditorOpen] = useState(false);
	const [cloneDialog, setCloneDialog] = useState<CloneDialogState | null>(null);
	const [loading, setLoading] = useState(false);
	const [message, setMessage] = useState<string | null>(null);
	const [error, setError] = useState<string | null>(null);

	const rootArg = useMemo(() => blankToNull(root), [root]);
	const tower1Arg = useMemo(() => blankToNull(tower1Url), [tower1Url]);
	const tower2Arg = useMemo(() => blankToNull(tower2Url), [tower2Url]);

	async function refresh() {
		setLoading(true);
		setError(null);
		try {
			const result = await invoke<VehicleSummary[]>("list_vehicles", {
				root: rootArg,
				tower1Url: tower1Arg,
				tower2Url: tower2Arg,
			});
			setVehicles(result);
			setMessage(
				`Loaded ${result.length} vehicle configuration${result.length === 1 ? "" : "s"}.`,
			);
			if (
				expandedKey &&
				!result.some((vehicle) => vehicle.key === expandedKey)
			) {
				setExpandedKey(null);
				setSelectedVehicle(null);
			}
		} catch (err) {
			setError(String(err));
		} finally {
			setLoading(false);
		}
	}

	async function toggleDetails(summary: VehicleSummary) {
		if (expandedKey === summary.key) {
			setExpandedKey(null);
			setSelectedVehicle(null);
			setEditorOpen(false);
			return;
		}
		setError(null);
		try {
			const vehicle = await invoke<VehicleConfig>("get_vehicle", {
				root: rootArg,
				key: summary.key,
			});
			setExpandedKey(summary.key);
			setSelectedVehicle(vehicle);
			setEditorText(JSON.stringify(vehicle, null, 2));
			setEditorOpen(false);
		} catch (err) {
			setError(String(err));
		}
	}

	async function saveEditedVehicle() {
		if (!selectedVehicle) return;
		setError(null);
		try {
			const parsed = JSON.parse(editorText) as VehicleConfig;
			const response = await invoke<CommandResponse<VehicleConfig>>(
				"save_vehicle",
				{
					root: rootArg,
					vehicle: parsed,
				},
			);
			setSelectedVehicle(response.value);
			setEditorText(JSON.stringify(response.value, null, 2));
			setMessage(validationMessage("Saved", response.validation));
			await refresh();
		} catch (err) {
			setError(String(err));
		}
	}

	async function disableVehicle(summary: VehicleSummary) {
		if (
			!window.confirm(
				`Disable ${summary.id}? The file will be marked inactive, not deleted.`,
			)
		) {
			return;
		}
		setError(null);
		try {
			const response = await invoke<CommandResponse<VehicleConfig>>(
				"disable_vehicle",
				{
					root: rootArg,
					key: summary.key,
				},
			);
			setMessage(
				validationMessage(`Disabled ${response.value.id}`, response.validation),
			);
			await refresh();
			if (expandedKey === summary.key) {
				setSelectedVehicle(response.value);
				setEditorText(JSON.stringify(response.value, null, 2));
			}
		} catch (err) {
			setError(String(err));
		}
	}

	async function cloneVehicle() {
		if (!cloneDialog) return;
		setError(null);
		const options: CloneOptions = {
			new_id: cloneDialog.newId.trim(),
			channel: blankToNull(cloneDialog.channel),
			profile: blankToNull(cloneDialog.profile),
		};
		try {
			const response = await invoke<CommandResponse<VehicleConfig>>(
				"clone_vehicle",
				{
					root: rootArg,
					sourceKey: cloneDialog.source.key,
					options,
				},
			);
			setCloneDialog(null);
			setMessage(
				validationMessage(`Cloned ${response.value.id}`, response.validation),
			);
			await refresh();
		} catch (err) {
			setError(String(err));
		}
	}

	useEffect(() => {
		void refresh();
		// Run initial discovery only once. Users can refresh after changing URLs/root.
		// eslint-disable-next-line react-hooks/exhaustive-deps
	}, []);

	return (
		<main className="app-shell">
			<header className="hero">
				<div>
					<p className="eyebrow">SUMO local-first desktop tool</p>
					<h1>Vehicle Configuration GUI</h1>
					<p className="hero-copy">
						Browse JSON/YAML vehicle configs, expand component details, clone
						existing setups, disable drafts, and check read-only linkage against
						Tower 1 and Tower 2.
					</p>
				</div>
				<button
					className="primary"
					onClick={() => void refresh()}
					disabled={loading}
				>
					{loading ? "Refreshing…" : "Refresh"}
				</button>
			</header>

			<section className="panel controls">
				<label>
					Config root
					<input
						value={root}
						placeholder="Auto-detect examples/managed-cvc-tower"
						onChange={(event) => setRoot(event.target.value)}
					/>
				</label>
				<label>
					Tower 1 URL
					<input
						value={tower1Url}
						onChange={(event) => setTower1Url(event.target.value)}
					/>
				</label>
				<label>
					Tower 2 URL
					<input
						value={tower2Url}
						onChange={(event) => setTower2Url(event.target.value)}
					/>
				</label>
			</section>

			{message && <div className="notice success">{message}</div>}
			{error && <div className="notice error">{error}</div>}

			<section className="panel">
				<div className="panel-heading">
					<h2>Available vehicles</h2>
					<span>{vehicles.length} configs</span>
				</div>
				<div className="table-wrap">
					<table>
						<thead>
							<tr>
								<th>Vehicle</th>
								<th>Channel/Profile</th>
								<th>Schema</th>
								<th>Components</th>
								<th>Tower linkage</th>
								<th>State</th>
								<th>Actions</th>
							</tr>
						</thead>
						<tbody>
							{vehicles.map((vehicle) => (
								<VehicleRow
									key={vehicle.key}
									summary={vehicle}
									expanded={expandedKey === vehicle.key}
									details={expandedKey === vehicle.key ? selectedVehicle : null}
									editorOpen={editorOpen && expandedKey === vehicle.key}
									editorText={editorText}
									onEditorText={setEditorText}
									onToggle={() => void toggleDetails(vehicle)}
									onEdit={() => setEditorOpen((open) => !open)}
									onSave={() => void saveEditedVehicle()}
									onClone={() =>
										setCloneDialog({
											source: vehicle,
											newId: `${vehicle.id}-copy`,
											channel: vehicle.channel,
											profile: vehicle.profile,
										})
									}
									onDisable={() => void disableVehicle(vehicle)}
								/>
							))}
							{vehicles.length === 0 && (
								<tr>
									<td colSpan={7} className="empty">
										No configs found. Set a config root or add vehicle.json /
										YAML profile files.
									</td>
								</tr>
							)}
						</tbody>
					</table>
				</div>
			</section>

			{cloneDialog && (
				<div className="modal-backdrop" role="presentation">
					<div
						className="modal"
						role="dialog"
						aria-modal="true"
						aria-label="Clone vehicle"
					>
						<h2>Clone {cloneDialog.source.id}</h2>
						<label>
							New vehicle id
							<input
								value={cloneDialog.newId}
								onChange={(event) =>
									setCloneDialog({ ...cloneDialog, newId: event.target.value })
								}
							/>
						</label>
						<label>
							Channel
							<input
								value={cloneDialog.channel}
								onChange={(event) =>
									setCloneDialog({
										...cloneDialog,
										channel: event.target.value,
									})
								}
							/>
						</label>
						<label>
							Profile
							<input
								value={cloneDialog.profile}
								onChange={(event) =>
									setCloneDialog({
										...cloneDialog,
										profile: event.target.value,
									})
								}
							/>
						</label>
						<div className="modal-actions">
							<button onClick={() => setCloneDialog(null)}>Cancel</button>
							<button
								className="primary"
								onClick={() => void cloneVehicle()}
								disabled={!cloneDialog.newId.trim()}
							>
								Create clone
							</button>
						</div>
					</div>
				</div>
			)}
		</main>
	);
}

function VehicleRow(props: {
	summary: VehicleSummary;
	expanded: boolean;
	details: VehicleConfig | null;
	editorOpen: boolean;
	editorText: string;
	onEditorText: (value: string) => void;
	onToggle: () => void;
	onEdit: () => void;
	onSave: () => void;
	onClone: () => void;
	onDisable: () => void;
}) {
	const { summary } = props;
	return (
		<>
			<tr className={summary.disabled ? "disabled-row" : ""}>
				<td>
					<button className="link-button" onClick={props.onToggle}>
						{props.expanded ? "▾" : "▸"} {summary.id}
					</button>
					<div className="subtle">{summary.kind || "vehicle"}</div>
				</td>
				<td>
					<strong>{summary.channel}</strong>
					<span className="subtle"> / {summary.profile}</span>
				</td>
				<td>
					<span className="badge neutral">{summary.schema}</span>
				</td>
				<td>
					{summary.component_count} components / {summary.part_count} parts
				</td>
				<td className="badge-list">
					<StatusBadge label="T1" status={summary.linkage?.tower1_device} />
					<StatusBadge label="T2" status={summary.linkage?.tower2_channel} />
				</td>
				<td>
					{summary.disabled ? (
						<span className="badge warning">disabled</span>
					) : (
						<span className="badge ok">active</span>
					)}
				</td>
				<td className="actions">
					<button onClick={props.onClone}>Clone</button>
					<button onClick={props.onDisable} disabled={summary.disabled}>
						Disable
					</button>
				</td>
			</tr>
			{props.expanded && (
				<tr className="details-row">
					<td colSpan={7}>
						{props.details ? (
							<VehicleDetails
								vehicle={props.details}
								editorOpen={props.editorOpen}
								editorText={props.editorText}
								onEditorText={props.onEditorText}
								onEdit={props.onEdit}
								onSave={props.onSave}
							/>
						) : (
							<div className="empty">Loading details…</div>
						)}
					</td>
				</tr>
			)}
		</>
	);
}

function VehicleDetails(props: {
	vehicle: VehicleConfig;
	editorOpen: boolean;
	editorText: string;
	onEditorText: (value: string) => void;
	onEdit: () => void;
	onSave: () => void;
}) {
	const { vehicle } = props;
	return (
		<div className="details">
			<div className="details-header">
				<div>
					<h3>{vehicle.id}</h3>
					<p>{vehicle.source_path}</p>
				</div>
				<div className="actions">
					<button onClick={props.onEdit}>
						{props.editorOpen ? "Close editor" : "Edit normalized JSON"}
					</button>
					{props.editorOpen && (
						<button className="primary" onClick={props.onSave}>
							Save
						</button>
					)}
				</div>
			</div>

			<div className="detail-grid">
				<InfoCard title="Deployment" data={vehicle.deployment} />
				<InfoCard title="Target" data={vehicle.target} />
				<InfoCard title="Labels" data={vehicle.labels} />
			</div>

			{props.editorOpen && (
				<textarea
					className="editor"
					value={props.editorText}
					onChange={(event) => props.onEditorText(event.target.value)}
					spellCheck={false}
				/>
			)}

			<h4>Components</h4>
			<div className="component-list">
				{vehicle.components.map((component) => (
					<details key={component.path} open>
						<summary>
							<strong>{component.path}</strong>
							<span>{component.kind || "component"}</span>
							{component.update_mode && (
								<span className="badge neutral">{component.update_mode}</span>
							)}
						</summary>
						<div className="part-list">
							{component.parts.map((part) => (
								<div className="part" key={`${component.path}:${part.id}`}>
									<span>{part.id}</span>
									<span>{part.kind || "file"}</span>
									<code>{part.source || "—"}</code>
								</div>
							))}
							{component.parts.length === 0 && (
								<div className="empty">No parts configured.</div>
							)}
						</div>
					</details>
				))}
			</div>
		</div>
	);
}

function InfoCard({ title, data }: { title: string; data: unknown }) {
	return (
		<div className="info-card">
			<h4>{title}</h4>
			<pre>{JSON.stringify(data, null, 2)}</pre>
		</div>
	);
}

function StatusBadge({
	label,
	status,
}: {
	label: string;
	status?: LinkStatus | null;
}) {
	const state = status?.state ?? "unknown";
	return (
		<span
			className={`badge ${badgeClass(state)}`}
			title={status?.message ?? state}
		>
			{label}: {state}
		</span>
	);
}

function badgeClass(state: string) {
	if (state === "available") return "ok";
	if (state === "missing") return "warning";
	if (state === "skipped") return "neutral";
	return "error";
}

function blankToNull(value: string): string | null {
	const trimmed = value.trim();
	return trimmed.length === 0 ? null : trimmed;
}

function validationMessage(prefix: string, validation: ValidationResult) {
	const warnings = validation.warnings.length
		? ` Warnings: ${validation.warnings.join("; ")}`
		: "";
	return `${prefix}.${warnings}`;
}
