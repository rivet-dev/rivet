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

// src/node_modules/@fortawesome/free-brands-svg-icons/faEdgeLegacy.js
var require_faEdgeLegacy = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faEdgeLegacy.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "edge-legacy";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e078";
    var svgPathData = "M-6 228.2l.4-.5c0 .2 0 .3-.1 .5l-.3 0zm460.6 15.5c0-44-7.8-84.5-28.8-122.4-41-73.4-113.6-113.3-198.6-113.3-139.9-.3-218.3 105.2-232.8 219.7 42.4-61.3 117.1-121.4 220.4-125 0 0 109.7 0 99.4 105l-175.8 0c6.4-37.4 18.5-59 34.3-78.9-75 34.9-121.8 96.1-120.8 188.3 .8 71.5 50.1 144.8 120.8 172 83.4 31.8 192.8 7.2 240.1-21.3l0-104.4c-80.9 56.5-270.9 60.9-272.3-67.6l314.1 0 0-52.1z";
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
    exports.faEdgeLegacy = exports.definition;
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

// src/_entries/faEdgeLegacy.js
var import_faEdgeLegacy = __toESM(require_faEdgeLegacy());
var export_faEdgeLegacy = import_faEdgeLegacy.definition;
export {
  export_faEdgeLegacy as faEdgeLegacy
};
