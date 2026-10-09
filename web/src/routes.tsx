import { Fragment } from "react";
import { Route, Routes } from "react-router-dom";

const PRESERVED = import.meta.glob("/src/pages/(_app|_404).tsx", {
  eager: true,
}) as Record<string, any>;
const ROUTES = import.meta.glob("/src/pages/**/[a-z[]*.tsx", {
  eager: true,
}) as Record<string, any>;

const preserved: Record<string, any> = Object.keys(PRESERVED).reduce(
  (acc, file) => ({
    ...acc,
    [file.replace(/\/src\/pages\/|\.tsx$/g, "")]: PRESERVED[file].default,
  }),
  {},
);

const routes = Object.keys(ROUTES).map((route) => {
  const path = route
    .replace(/\/src\/pages|index|\.tsx$/g, "")
    .replace(/\[\.{3}.+\]/, "*")
    .replace(/\[(.+)\]/, ":$1");
  return { path: path || "/", component: ROUTES[route].default };
});

function App() {
  const App = preserved["_app"] || Fragment;
  const NotFound = preserved["_404"] || Fragment;
  return (
    <App>
      <Routes>
        {routes.map(({ path, component: Component = Fragment }) => (
          <Route key={path} path={path} element={<Component />} />
        ))}
        <Route path="*" element={<NotFound />} />
      </Routes>
    </App>
  );
}

export default App;
