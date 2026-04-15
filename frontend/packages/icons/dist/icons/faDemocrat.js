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

// src/node_modules/@fortawesome/free-solid-svg-icons/faDemocrat.js
var require_faDemocrat = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faDemocrat.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "democrat";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f747";
    var svgPathData = "M64 32c0-8.9 3.8-20.9 6.2-27.3 1-2.8 3.8-4.7 6.8-4.7 1.9 0 3.8 .7 5.2 2.1L128 45.7 173.8 2.1c1.4-1.3 3.2-2.1 5.2-2.1 3 0 5.8 1.8 6.8 4.7 2.4 6.5 6.2 18.4 6.2 27.3 0 26.5-21.9 42-29.5 46.6l76.2 72.6c6 5.7 13.9 8.8 22.1 8.8l219.2 0 32 0c40.3 0 78.2 19 102.4 51.2l19.2 25.6c10.6 14.1 7.7 34.2-6.4 44.8s-34.2 7.7-44.8-6.4l-19.2-25.6c-5.3-7-11.8-12.8-19.2-17l0 87.4-352 0-40.4-94.3c-3.9-9.2-15.3-12.6-23.6-7l-42.1 28c-9.1 6.1-19.7 9.3-30.7 9.3l-2 0C23.9 256 0 232.1 0 202.7 0 190.6 4.1 178.9 11.7 169.4L87.6 74.6C78.1 67.4 64 53.2 64 32zM544 352l0 128c0 17.7-14.3 32-32 32l-32 0c-17.7 0-32-14.3-32-32l0-64-160 0 0 64c0 17.7-14.3 32-32 32l-32 0c-17.7 0-32-14.3-32-32l0-128 352 0zM256 280a24 24 0 1 0 0-48 24 24 0 1 0 0 48zm248-24a24 24 0 1 0 -48 0 24 24 0 1 0 48 0zM368 280a24 24 0 1 0 0-48 24 24 0 1 0 0 48z";
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
    exports.faDemocrat = exports.definition;
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

// src/_entries/faDemocrat.js
var import_faDemocrat = __toESM(require_faDemocrat());
var export_faDemocrat = import_faDemocrat.definition;
export {
  export_faDemocrat as faDemocrat
};
