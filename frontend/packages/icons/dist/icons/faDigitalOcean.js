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

// src/node_modules/@fortawesome/free-brands-svg-icons/faDigitalOcean.js
var require_faDigitalOcean = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faDigitalOcean.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "digital-ocean";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f391";
    var svgPathData = "M87 481.8l73.7 0 0-73.6-73.7 0 0 73.6zM25.4 346.6l0 61.6 61.6 0 0-61.6-61.6 0zM491.6 176.9C468.6 102.7 409.2 43.6 335 20.3 164.9-32.8 8 93.7 8 255.9l95.8 0c0-101.8 101-180.5 208.1-141.7 39.7 14.3 71.5 46.1 85.8 85.7 39.1 107-39.7 207.8-141.4 208l0 .3-.3 0 0 95.8c162.6 0 288.8-156.8 235.6-327.1zm-235.3 231l0-95.3-95.6 0 0 95.6 95.3 0 0-.3 .3 0z";
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
    exports.faDigitalOcean = exports.definition;
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

// src/_entries/faDigitalOcean.js
var import_faDigitalOcean = __toESM(require_faDigitalOcean());
var export_faDigitalOcean = import_faDigitalOcean.definition;
export {
  export_faDigitalOcean as faDigitalOcean
};
