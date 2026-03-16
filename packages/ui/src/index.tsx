type Metric = {
  label: string;
  value: string;
};

type StarterSurfaceProps = {
  title: string;
  subtitle: string;
  metrics: Metric[];
};

export function StarterSurface({ title, subtitle, metrics }: StarterSurfaceProps) {
  return (
    <section className="starter-surface">
      <h2>{title}</h2>
      <p>{subtitle}</p>
      <div className="metric-grid">
        {metrics.map((metric) => (
          <div className="metric" key={metric.label}>
            <span className="metric-label">{metric.label}</span>
            <strong className="metric-value">{metric.value}</strong>
          </div>
        ))}
      </div>
    </section>
  );
}
