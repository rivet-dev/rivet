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

// src/node_modules/@fortawesome/free-solid-svg-icons/faTransgender.js
var require_faTransgender = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faTransgender.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "transgender";
    var width = 576;
    var height = 512;
    var aliases = [9895, "transgender-alt"];
    var unicode = "f225";
    var svgPathData = "M128-32c17.7 0 32 14.3 32 32s-14.3 32-32 32L97.9 32 136 70.1 151 55c9.4-9.4 24.6-9.4 33.9 0s9.4 24.6 0 33.9l-15 15 14.2 14.2c27.9-23.8 64.2-38.2 103.8-38.2 36.7 0 70.6 12.4 97.6 33.2L466.7 32 448 32c-17.7 0-32-14.3-32-32s14.3-32 32-32l96 0c17.7 0 32 14.3 32 32l0 96c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-18.7-84.4 84.4c13 23.1 20.4 49.9 20.4 78.3 0 77.4-55 142-128 156.8l0 35.2 32 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l-32 0 0 16c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-16-32 0c-17.7 0-32-14.3-32-32s14.3-32 32-32l32 0 0-35.2c-73-14.8-128-79.4-128-156.8 0-31.4 9-60.7 24.7-85.4l-16.7-16.7-15 15c-9.4 9.4-24.6 9.4-33.9 0s-9.4-24.6 0-33.9l15-15-38.1-38.1 0 30.1c0 17.7-14.3 32-32 32S0 113.7 0 96L0 0C0-17.7 14.3-32 32-32l96 0zM288 336a96 96 0 1 0 0-192 96 96 0 1 0 0 192z";
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
    exports.faTransgender = exports.definition;
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

// src/_entries/faTransgenderAlt.js
var import_faTransgender = __toESM(require_faTransgender());
var export_faTransgenderAlt = import_faTransgender.definition;
export {
  export_faTransgenderAlt as faTransgenderAlt
};
