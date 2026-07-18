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

// src/node_modules/@fortawesome/free-brands-svg-icons/faPeriscope.js
var require_faPeriscope = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faPeriscope.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "periscope";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f3da";
    var svgPathData = "M370 63.6c-38.6-41-89.5-63.6-143.4-63.6-114.7 0-208.1 96.2-208.1 214.4 0 75.1 57.8 159.8 82.7 192.7 36.6 48.4 91.4 104.9 125.4 104.9 41.6 0 112.9-94.2 120.9-105 24.6-33.1 82-118.3 82-192.6 0-56.5-21.1-110.1-59.5-150.8zM226.6 493.9c-42.5 0-190-167.3-190-279.4 0-107.4 83.9-196.3 190-196.3 100.8 0 184.7 89 184.7 196.3 .1 112.1-147.4 279.4-184.7 279.4zM338 206.8c0 59.1-51.1 109.7-110.8 109.7-100.6 0-150.7-108.2-92.9-181.8l0 .4c0 24.5 20.1 44.4 44.8 44.4s44.8-19.9 44.8-44.4c0-18.2-11.1-33.8-26.9-40.7 76.6-19.2 141 39.3 141 112.4z";
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
    exports.faPeriscope = exports.definition;
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

// src/_entries/faPeriscope.js
var import_faPeriscope = __toESM(require_faPeriscope());
var export_faPeriscope = import_faPeriscope.definition;
export {
  export_faPeriscope as faPeriscope
};
