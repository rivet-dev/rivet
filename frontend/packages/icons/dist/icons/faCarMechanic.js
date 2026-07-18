var __create = Object.create;
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __getProtoOf = Object.getPrototypeOf;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(
  // If the importer is in node compatibility mode or this is not an ESM
  // file that has been converted to a CommonJS file using a Babel-
  // compatible transform (i.e. "__esModule" has not been set), then set
  // "default" to the CommonJS "module.exports" for node compatibility.
  isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target,
  mod
));

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCarWrench.js
var require_faCarWrench = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCarWrench.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "car-wrench";
    var width = 512;
    var height = 512;
    var aliases = ["car-mechanic"];
    var unicode = "f5e3";
    var svgPathData = "M468.9 104L456 104l-.5 0c-.7 0-1.4-.1-2.1-.1C441.3 102.5 432 92.4 432 80s9.3-22.5 21.3-23.9c.7-.1 1.4-.1 2.1-.1l.5 0 12.9 0 17.6 0c10.8 0 18.4-10.6 12.5-19.6c-3.6-5.5-7.8-10.5-12.5-14.9c-4.2-3.9-8.8-7.4-13.8-10.3C470.8 10 468.9 9 466.9 8C456.4 2.9 444.5 0 432 0c-29.1 0-54.5 15.5-68.5 38.7c-3.3 5.4-8.9 9.3-15.3 9.3L163.8 48c-6.4 0-12-3.8-15.3-9.3C134.5 15.5 109.1 0 80 0C67.5 0 55.6 2.9 45.1 8c-2 1-3.9 2-5.8 3.1c-5 2.9-9.6 6.4-13.8 10.3c-4.7 4.4-9 9.4-12.5 14.9C7 45.4 14.7 56 25.5 56l17.6 0L56 56l.5 0c.7 0 1.4 .1 2.1 .1C70.7 57.5 80 67.6 80 80s-9.3 22.5-21.3 23.9c-.7 .1-1.4 .1-2.1 .1l-.5 0-12.9 0-17.6 0C14.7 104 7 114.6 12.9 123.6c3.6 5.5 7.8 10.5 12.5 14.9c4.2 3.9 8.8 7.4 13.8 10.3c1.9 1.1 3.8 2.2 5.8 3.1c10.6 5.1 22.4 8 34.9 8c29.1 0 54.5-15.5 68.5-38.7c3.3-5.4 8.9-9.3 15.3-9.3l184.4 0c6.4 0 12 3.8 15.3 9.3c14 23.2 39.5 38.7 68.5 38.7c12.5 0 24.4-2.9 34.9-8c2-1 3.9-2 5.8-3.1c5-2.9 9.6-6.4 13.8-10.3c4.7-4.4 9-9.4 12.5-14.9c5.9-9-1.7-19.6-12.5-19.6l-17.6 0zM185.8 224l140.3 0c6.8 0 12.8 4.3 15.1 10.6L360.3 288l-208.6 0 19.1-53.4c2.3-6.4 8.3-10.6 15.1-10.6zm-75.3-10.9L82.2 292.4C62.1 300.9 48 320.8 48 344l0 40 0 64 0 32c0 17.7 14.3 32 32 32l16 0c17.7 0 32-14.3 32-32l0-32 256 0 0 32c0 17.7 14.3 32 32 32l16 0c17.7 0 32-14.3 32-32l0-32 0-64 0-40c0-23.2-14.1-43.1-34.2-51.6l-28.3-79.3C390.1 181.3 360 160 326.2 160l-140.3 0c-33.8 0-64 21.3-75.3 53.1zM128 344a24 24 0 1 1 0 48 24 24 0 1 1 0-48zm232 24a24 24 0 1 1 48 0 24 24 0 1 1 -48 0z";
    exports.definition = {
      prefix,
      iconName,
      icon: [
        width,
        height,
        aliases,
        unicode,
        svgPathData
      ]
    };
    exports.faCarWrench = exports.definition;
    exports.prefix = prefix;
    exports.iconName = iconName;
    exports.width = width;
    exports.height = height;
    exports.ligatures = aliases;
    exports.unicode = unicode;
    exports.svgPathData = svgPathData;
    exports.aliases = aliases;
  }
});

// src/_entries/faCarMechanic.js
var import_faCarWrench = __toESM(require_faCarWrench());
var export_faCarMechanic = import_faCarWrench.definition;
export {
  export_faCarMechanic as faCarMechanic
};
