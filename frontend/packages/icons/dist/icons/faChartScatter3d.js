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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faChartScatter3d.js
var require_faChartScatter3d = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faChartScatter3d.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "chart-scatter-3d";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e0e8";
    var svgPathData = "M32 96a32 32 0 1 0 0-64 32 32 0 1 0 0 64zM288 64c0-17.7-14.3-32-32-32s-32 14.3-32 32l0 209L43.5 423.4c-13.6 11.3-15.4 31.5-4.1 45.1s31.5 15.4 45.1 4.1L256 329.7 427.5 472.6c13.6 11.3 33.8 9.5 45.1-4.1s9.5-33.8-4.1-45.1L288 273l0-209zm96 32a32 32 0 1 0 0-64 32 32 0 1 0 0 64zm32 160a32 32 0 1 0 -64 0 32 32 0 1 0 64 0zm64-64a32 32 0 1 0 0-64 32 32 0 1 0 0 64zM160 160a32 32 0 1 0 -64 0 32 32 0 1 0 64 0zM64 256A32 32 0 1 0 0 256a32 32 0 1 0 64 0zM256 480a32 32 0 1 0 0-64 32 32 0 1 0 0 64z";
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
    exports.faChartScatter3d = exports.definition;
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

// src/_entries/faChartScatter3d.js
var import_faChartScatter3d = __toESM(require_faChartScatter3d());
var export_faChartScatter3d = import_faChartScatter3d.definition;
export {
  export_faChartScatter3d as faChartScatter3d
};
