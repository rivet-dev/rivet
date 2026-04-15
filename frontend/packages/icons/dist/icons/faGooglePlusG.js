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

// src/node_modules/@fortawesome/free-brands-svg-icons/faGooglePlusG.js
var require_faGooglePlusG = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faGooglePlusG.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "google-plus-g";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f0d5";
    var svgPathData = "M386.3 228.5c1.8 9.7 3.1 19.4 3.1 32 0 109.8-73.6 187.5-184.4 187.5-106.1 0-192-85.9-192-192S98.9 64 205 64c51.9 0 95.1 18.9 128.6 50.3l-52.1 50c-14.1-13.6-39-29.6-76.5-29.6-65.5 0-118.9 54.2-118.9 121.3S139.5 377.3 205 377.3c76 0 104.5-54.7 109-82.8l-109 0 0-66 181.3 0 0 0zm185.4 6.4l0-55.7-56 0 0 55.7-55.7 0 0 56 55.7 0 0 55.7 56 0 0-55.7 55.7 0 0-56-55.7 0z";
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
    exports.faGooglePlusG = exports.definition;
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

// src/_entries/faGooglePlusG.js
var import_faGooglePlusG = __toESM(require_faGooglePlusG());
var export_faGooglePlusG = import_faGooglePlusG.definition;
export {
  export_faGooglePlusG as faGooglePlusG
};
