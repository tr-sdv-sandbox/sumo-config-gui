import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

type JsonMap = Record<string, unknown>;

interface Deployment {
	channel: string;
	profile: string;
}

interface TargetTypeConfig {
	name: string;
	kind: string;
	description?: string | null;
}

interface PartConfig {
	id: string;
	kind: string;
	source?: string | null;
}

interface ComponentConfig {
	path: string;
	parent_path?: string | null;
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
	target_type: TargetTypeConfig;
	deployment: Deployment;
	target: JsonMap;
	labels: JsonMap;
	config_snapshot?: unknown | null;
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
	tower2_channel: LinkStatus;
}

interface VehicleSummary {
	key: string;
	id: string;
	kind: string;
	target_type: string;
	channel: string;
	profile: string;
	schema: string;
	source_path: string;
	disabled: boolean;
	component_count: number;
	part_count: number;
	linkage?: TowerLinkage | null;
}

interface Tower1Config {
	id: string;
	model?: string | null;
	status: string;
	cert_serial?: string | null;
	cert_not_after?: string | null;
	cert_fingerprint?: string | null;
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
	target_type?: string | null;
}

interface CloneDialogState {
	source: VehicleSummary;
	newId: string;
	channel: string;
	profile: string;
	targetType: string;
}

interface LaunchConfig {
	config_root?: string | null;
	tower1_url: string;
	tower2_url: string;
}

type ActiveTab = "tower2" | "tower1";

type TowerLinks = Record<string, string[]>;

const TOWER_LINKS_STORAGE_KEY = "sumo-config-gui:tower-links:v1";
const DEFAULT_TOWER1 = "http://localhost:8080";
const DEFAULT_TOWER2 = "http://localhost:8081";

export default function App() {
	const [activeTab, setActiveTab] = useState<ActiveTab>("tower2");
	const [root, setRoot] = useState("");
	const [tower1Url, setTower1Url] = useState(DEFAULT_TOWER1);
	const [tower2Url, setTower2Url] = useState(DEFAULT_TOWER2);
	const [selectedHubConfigKey, setSelectedHubConfigKey] = useState("all");
	const [vehicles, setVehicles] = useState<VehicleSummary[]>([]);
	const [tower1Configs, setTower1Configs] = useState<Tower1Config[]>([]);
	const [towerLinks, setTowerLinks] = useState<TowerLinks>(() =>
		loadTowerLinks(),
	);
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
	const linkedTower1Ids = useMemo(
		() => new Set(Object.values(towerLinks).flat()),
		[towerLinks],
	);
	const filteredVehicles = useMemo(
		() =>
			selectedHubConfigKey === "all"
				? vehicles
				: vehicles.filter((vehicle) => vehicle.key === selectedHubConfigKey),
		[selectedHubConfigKey, vehicles],
	);

	async function refresh(rootOverride?: string, tower2Override?: string) {
		setLoading(true);
		setError(null);
		try {
			const result = await invoke<VehicleSummary[]>("list_vehicles", {
				root: rootOverride === undefined ? rootArg : blankToNull(rootOverride),
				tower2Url:
					tower2Override === undefined
						? tower2Arg
						: blankToNull(tower2Override),
			});
			setVehicles(result);
			setMessage(
				`Loaded ${result.length} target configuration${result.length === 1 ? "" : "s"}.`,
			);
			if (
				selectedHubConfigKey !== "all" &&
				!result.some((vehicle) => vehicle.key === selectedHubConfigKey)
			) {
				setSelectedHubConfigKey("all");
			}
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

	async function refreshTower1(tower1Override?: string) {
		setLoading(true);
		setError(null);
		try {
			const result = await invoke<Tower1Config[]>("list_tower1_configs", {
				tower1Url:
					tower1Override === undefined
						? tower1Arg
						: blankToNull(tower1Override),
			});
			setTower1Configs(result);
			setMessage(
				`Loaded ${result.length} CA configuration${result.length === 1 ? "" : "s"}.`,
			);
		} catch (err) {
			setError(String(err));
		} finally {
			setLoading(false);
		}
	}

	async function refreshActive() {
		if (activeTab === "tower1") {
			await refreshTower1();
			return;
		}
		await refresh();
	}

	function setTargetTower1Link(targetKey: string, tower1Id: string, linked: boolean) {
		setTowerLinks((current) => {
			const existing = current[targetKey] ?? [];
			const nextIds = linked
				? Array.from(new Set([...existing, tower1Id])).sort()
				: existing.filter((id) => id !== tower1Id);
			const next = { ...current };
			if (nextIds.length) {
				next[targetKey] = nextIds;
			} else {
				delete next[targetKey];
			}
			storeTowerLinks(next);
			return next;
		});
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
				tower2Url: tower2Arg,
				key: summary.key,
			});
			setExpandedKey(summary.key);
			setSelectedVehicle(vehicle);
			setEditorText(editableConfigText(vehicle));
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
			setEditorText(editableConfigText(response.value));
			setMessage(validationMessage("Saved", response.validation));
			await refresh();
		} catch (err) {
			setError(String(err));
		}
	}

	async function disableVehicle(summary: VehicleSummary) {
		if (
			!window.confirm(
				`Disable target config ${targetConfigLabel(summary)}? The file will be marked inactive, not deleted.`,
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
				validationMessage("Disabled target config", response.validation),
			);
			await refresh();
			if (expandedKey === summary.key) {
				setSelectedVehicle(response.value);
				setEditorText(editableConfigText(response.value));
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
			target_type: blankToNull(cloneDialog.targetType),
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
				validationMessage("Cloned target config", response.validation),
			);
			await refresh();
		} catch (err) {
			setError(String(err));
		}
	}

	useEffect(() => {
		async function loadLaunchConfig() {
			try {
				const config = await invoke<LaunchConfig>("launch_config");
				const nextRoot = config.config_root ?? "";
				const nextTower1 = config.tower1_url || DEFAULT_TOWER1;
				const nextTower2 = config.tower2_url || DEFAULT_TOWER2;
				setRoot(nextRoot);
				setTower1Url(nextTower1);
				setTower2Url(nextTower2);
				await Promise.all([refresh(nextRoot, nextTower2), refreshTower1(nextTower1)]);
			} catch {
				await Promise.all([refresh(), refreshTower1()]);
			}
		}

		void loadLaunchConfig();
		// Run initial discovery only once. Users can refresh after changing URLs/root.
		// eslint-disable-next-line react-hooks/exhaustive-deps
	}, []);

	return (
		<main className="app-shell">
			<header className="hero">
				<div>
					<p className="eyebrow">SUMO CA/HUB configuration tool</p>
					<h1>Target Configuration GUI</h1>
					<p className="hero-copy">
						Browse HUB target releases by default, inspect CA identity configs
						separately, and connect one HUB config to multiple CA configs.
					</p>
				</div>
				<button
					className="primary"
					onClick={() => void refreshActive()}
					disabled={loading}
				>
					{loading ? "Refreshing…" : "Refresh"}
				</button>
			</header>

			<section className="panel controls">
				<label>
					Local test config root
					<input
						value={root}
						placeholder="Optional: file-mode smoke testing only"
						onChange={(event) => setRoot(event.target.value)}
					/>
				</label>
				<label>
					CA URL
					<input
						value={tower1Url}
						onChange={(event) => setTower1Url(event.target.value)}
					/>
				</label>
				<label>
					HUB URL
					<input
						value={tower2Url}
						onChange={(event) => setTower2Url(event.target.value)}
					/>
				</label>
			</section>

			{message && <div className="notice success">{message}</div>}
			{error && <div className="notice error">{error}</div>}

			<nav className="tabs" aria-label="Configuration views">
				<button
					className={activeTab === "tower2" ? "active" : ""}
					onClick={() => setActiveTab("tower2")}
				>
					HUB target releases
				</button>
				<button
					className={activeTab === "tower1" ? "active" : ""}
					onClick={() => setActiveTab("tower1")}
				>
					CA configs
					<span className="badge neutral">{tower1Configs.length}</span>
				</button>
			</nav>

			{activeTab === "tower2" && <section className="panel">
				<div className="panel-heading">
					<div>
						<h2>Available HUB configurations</h2>
						<span>
							{vehicles.length} configs / {linkedTower1Ids.size} linked CA configs
						</span>
					</div>
					<label className="inline-select">
						Switch HUB config
						<select
							value={selectedHubConfigKey}
							onChange={(event) => setSelectedHubConfigKey(event.target.value)}
						>
							<option value="all">All HUB configs</option>
							{vehicles.map((vehicle) => (
								<option key={vehicle.key} value={vehicle.key}>
									{targetConfigLabel(vehicle)} ({vehicle.channel})
								</option>
							))}
						</select>
					</label>
				</div>
				<div className="table-wrap">
					<table>
						<thead>
							<tr>
								<th>Target config</th>
								<th>Channel/Profile</th>
								<th>Schema</th>
								<th>Components</th>
								<th>Target release</th>
								<th>State</th>
								<th>Actions</th>
							</tr>
						</thead>
						<tbody>
							{filteredVehicles.map((vehicle) => (
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
											newId: cloneConfigName(vehicle),
											channel: vehicle.channel,
											profile: vehicle.profile,
											targetType: vehicle.target_type,
										})
									}
									onDisable={() => void disableVehicle(vehicle)}
									linkedTower1Count={towerLinks[vehicle.key]?.length ?? 0}
								/>
							))}
							{filteredVehicles.length === 0 && (
								<tr>
									<td colSpan={7} className="empty">
										No HUB target releases found for this selection. For local
										testing, set a local test config root with vehicle.json / YAML
										profile files.
									</td>
								</tr>
							)}
						</tbody>
					</table>
				</div>
			</section>}

			{activeTab === "tower1" && (
				<Tower1Tab
					configs={tower1Configs}
					targets={vehicles}
					links={towerLinks}
					onLink={setTargetTower1Link}
				/>
			)}

			{cloneDialog && (
				<div className="modal-backdrop" role="presentation">
					<div
						className="modal"
						role="dialog"
						aria-modal="true"
						aria-label="Clone target config"
					>
						<h2>Clone target config {targetConfigLabel(cloneDialog.source)}</h2>
						<label>
							New config name
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
						<label>
							Target type
							<input
								value={cloneDialog.targetType}
								onChange={(event) =>
									setCloneDialog({
										...cloneDialog,
										targetType: event.target.value,
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
	linkedTower1Count: number;
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
						{props.expanded ? "▾" : "▸"} {targetConfigLabel(summary)}
					</button>
					<div className="subtle">{summary.source_path}</div>
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
					<StatusBadge label="HUB" status={summary.linkage?.tower2_channel} />
					{props.linkedTower1Count > 0 && (
						<span className="badge ok">CA links: {props.linkedTower1Count}</span>
					)}
				</td>
				<td>
					{summary.disabled ? (
						<span className="badge warning">disabled</span>
					) : (
						<span className="badge ok">active</span>
					)}
				</td>
				<td className="actions">
					{summary.schema === "tower2-release" ? (
						<span className="subtle">read-only</span>
					) : (
						<>
							<button onClick={props.onClone}>Clone config</button>
							<button onClick={props.onDisable} disabled={summary.disabled}>
								Disable
							</button>
						</>
					)}
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

function Tower1Tab({
	configs,
	targets,
	links,
	onLink,
}: {
	configs: Tower1Config[];
	targets: VehicleSummary[];
	links: TowerLinks;
	onLink: (targetKey: string, tower1Id: string, linked: boolean) => void;
}) {
	return (
		<section className="panel tower1-panel">
			<div className="panel-heading">
				<div>
					<h2>CA configurations</h2>
					<p className="subtle">
						Identity/device configs can be connected to any number of HUB target
						releases. Links are stored locally in this GUI.
					</p>
				</div>
				<span>{configs.length} configs</span>
			</div>
			<div className="tower1-grid">
				{configs.map((config) => (
					<div className="tower1-card" key={config.id}>
						<div className="details-header">
							<div>
								<h3>{config.id}</h3>
								<p>{config.model || "No model recorded"}</p>
							</div>
							<span className={`badge ${badgeClass(config.status)}`}>
								{config.status}
							</span>
						</div>
						<dl className="field-list compact">
							{config.cert_serial && (
								<div>
									<dt>Cert serial</dt>
									<dd>{config.cert_serial}</dd>
								</div>
							)}
							{config.cert_not_after && (
								<div>
									<dt>Expires</dt>
									<dd>{config.cert_not_after}</dd>
								</div>
							)}
							{config.cert_fingerprint && (
								<div>
									<dt>Fingerprint</dt>
									<dd>{config.cert_fingerprint}</dd>
								</div>
							)}
						</dl>
						<h4>Connected HUB configs</h4>
						<div className="target-link-list">
							{targets.map((target) => {
								const linked = links[target.key]?.includes(config.id) ?? false;
								return (
									<label className="check-row" key={`${config.id}:${target.key}`}>
										<input
											type="checkbox"
											checked={linked}
											onChange={(event) =>
												onLink(target.key, config.id, event.target.checked)
											}
										/>
										<span>
											<strong>{targetConfigLabel(target)}</strong>
											<span className="subtle"> {target.channel}</span>
										</span>
									</label>
								);
							})}
							{targets.length === 0 && (
								<div className="empty">Load HUB target releases to create links.</div>
							)}
						</div>
					</div>
				))}
				{configs.length === 0 && (
					<div className="empty">
						No CA configs found. Check the CA URL and refresh this tab.
					</div>
				)}
			</div>
		</section>
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
					<h3>{targetConfigTitle(vehicle)}</h3>
					<p>{vehicle.source_path}</p>
				</div>
				<div className="actions">
					{vehicle.schema === "tower2-release" ? (
						<span className="badge neutral">read-only HUB release</span>
					) : (
						<>
							<button onClick={props.onEdit}>
								{props.editorOpen ? "Close editor" : "Edit normalized JSON"}
							</button>
							{props.editorOpen && (
								<button className="primary" onClick={props.onSave}>
									Save
								</button>
							)}
						</>
					)}
				</div>
			</div>

			<div className="detail-grid">
				<InfoCard title="Target type" data={vehicle.target_type} />
				<InfoCard title="Deployment" data={vehicle.deployment} />
				<InfoCard title="Target" data={vehicle.target} />
				<InfoCard title="Labels" data={vehicle.labels} />
				{vehicle.config_snapshot != null && (
					<SnapshotDetails snapshot={vehicle.config_snapshot} />
				)}
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
							{component.parent_path && (
								<span className="badge neutral">
									workload of {component.parent_path}
								</span>
							)}
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

function SnapshotDetails({ snapshot }: { snapshot: unknown }) {
	const value = isRecord(snapshot) ? snapshot : null;
	const components = isRecord(value?.components) ? value.components : null;
	const labels = isRecord(value?.labels) ? value.labels : null;

	return (
		<div className="info-card snapshot-card">
			<h4>Config snapshot</h4>
			{value ? (
				<>
					<div className="detail-grid snapshot-grid">
						<SnapshotFields
							title="Release metadata"
							data={{ schema: value.schema, kind: value.kind, id: value.id }}
						/>
						<InfoCard
							title="Snapshot target type"
							data={value.target_type ?? {}}
						/>
						<InfoCard
							title="Snapshot deployment"
							data={value.deployment ?? {}}
						/>
						<div className="info-card">
							<h4>Snapshot labels</h4>
							{labels ? <LabelChips labels={labels} /> : <pre>{"{}"}</pre>}
						</div>
					</div>
					{components && (
						<div className="component-list">
							{Object.entries(components).map(([path, component]) => (
								<SnapshotComponent key={path} path={path} component={component} />
							))}
						</div>
					)}
				</>
			) : (
				<pre>{JSON.stringify(snapshot, null, 2)}</pre>
			)}
		</div>
	);
}

function SnapshotFields({ title, data }: { title: string; data: JsonMap }) {
	const entries = Object.entries(data).filter(([, value]) => value != null);
	return (
		<div className="info-card">
			<h4>{title}</h4>
			{entries.length ? (
				<dl className="field-list">
					{entries.map(([key, value]) => (
						<div key={key}>
							<dt>{labelize(key)}</dt>
							<dd>{String(value)}</dd>
						</div>
					))}
				</dl>
			) : (
				<pre>{"{}"}</pre>
			)}
		</div>
	);
}

function LabelChips({ labels }: { labels: JsonMap }) {
	const entries = Object.entries(labels);
	return entries.length ? (
		<div className="label-chips">
			{entries.map(([key, value]) => (
				<span className="badge neutral" key={key}>
					{key}: {String(value)}
				</span>
			))}
		</div>
	) : (
		<pre>{"{}"}</pre>
	);
}

function SnapshotComponent({
	path,
	component,
}: {
	path: string;
	component: unknown;
}) {
	const record = isRecord(component) ? component : null;
	const parts = Array.isArray(record?.parts) ? record.parts : [];
	const target = isRecord(record?.target) ? record.target : null;

	return (
		<details open>
			<summary>
				<strong>{path}</strong>
				<span>{snapshotComponentKind(component)}</span>
				{getString(record, "version") && (
					<span className="badge neutral">{getString(record, "version")}</span>
				)}
				{getString(record, "update_mode") && (
					<span className="badge neutral">{getString(record, "update_mode")}</span>
				)}
			</summary>
			{record ? (
				<div className="snapshot-component-body">
					<dl className="field-list compact">
						{getString(record, "parent_path") && (
							<div>
								<dt>Workload of</dt>
								<dd>{getString(record, "parent_path")}</dd>
							</div>
						)}
						{target &&
							Object.entries(target).map(([key, value]) => (
								<div key={key}>
									<dt>{labelize(key)}</dt>
									<dd>{String(value)}</dd>
								</div>
							))}
					</dl>
					{parts.length ? (
						<div className="snapshot-parts">
							<div className="part header">
								<span>Part</span>
								<span>Kind</span>
								<span>Source</span>
							</div>
							{parts.map((part, index) => {
								const partRecord = isRecord(part) ? part : null;
								return (
									<div className="part" key={`${path}:part:${index}`}>
										<span>{getString(partRecord, "id", `part-${index + 1}`)}</span>
										<span>{getString(partRecord, "kind", "file")}</span>
										<code>{getString(partRecord, "source", "—")}</code>
									</div>
								);
							})}
						</div>
					) : (
						<div className="empty">No snapshot parts recorded.</div>
					)}
				</div>
			) : (
				<pre>{JSON.stringify(component, null, 2)}</pre>
			)}
		</details>
	);
}

function snapshotComponentKind(component: unknown) {
	return isRecord(component) && typeof component.kind === "string"
		? component.kind
		: "component";
}

function getString(record: JsonMap | null, key: string, fallback = "") {
	const value = record?.[key];
	return typeof value === "string" && value.length > 0 ? value : fallback;
}

function labelize(value: string) {
	return value.replace(/_/g, " ");
}

function isRecord(value: unknown): value is JsonMap {
	return typeof value === "object" && value !== null && !Array.isArray(value);
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
	if (state === "available" || state === "enrolled") return "ok";
	if (state === "missing" || state === "registered") return "warning";
	if (state === "skipped") return "neutral";
	return "error";
}

function loadTowerLinks(): TowerLinks {
	try {
		const raw = window.localStorage.getItem(TOWER_LINKS_STORAGE_KEY);
		if (!raw) return {};
		const parsed = JSON.parse(raw) as unknown;
		if (!isRecord(parsed)) return {};
		return Object.fromEntries(
			Object.entries(parsed)
				.map(([key, value]) => [
					key,
					Array.isArray(value)
						? value.filter((item): item is string => typeof item === "string")
						: [],
				])
				.filter(([, ids]) => ids.length > 0),
		);
	} catch {
		return {};
	}
}

function storeTowerLinks(links: TowerLinks) {
	window.localStorage.setItem(TOWER_LINKS_STORAGE_KEY, JSON.stringify(links));
}

function editableConfigText(vehicle: VehicleConfig) {
	const editable = { ...vehicle } as Partial<VehicleConfig>;
	delete editable.id;
	return JSON.stringify(editable, null, 2);
}

function targetConfigLabel(summary: VehicleSummary) {
	return [summary.target_type || "target", summary.profile || "default"]
		.filter(Boolean)
		.join(" / ");
}

function targetConfigTitle(vehicle: VehicleConfig) {
	return [
		vehicle.target_type.name || "target",
		vehicle.deployment.profile || "default",
	]
		.filter(Boolean)
		.join(" / ");
}

function cloneConfigName(summary: VehicleSummary) {
	return sanitizeSegment(
		[summary.channel, summary.target_type, summary.profile, "copy"]
			.filter(Boolean)
			.join("-"),
	);
}

function sanitizeSegment(value: string) {
	return value.replace(/[^a-zA-Z0-9._-]+/g, "-").replace(/^-+|-+$/g, "");
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
