import { Link } from "react-router-dom";

export default function PageNotFound() {
  return (
    <section className="upl">
      <h2>Page not found</h2>
      <div className="sub">That page does not exist.</div>
      <Link className="btn pri" to="/squad">
        Back to squad
      </Link>
    </section>
  );
}
