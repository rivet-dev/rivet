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

// src/node_modules/@fortawesome/free-brands-svg-icons/faMedapps.js
var require_faMedapps = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faMedapps.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "medapps";
    var width = 320;
    var height = 512;
    var aliases = [];
    var unicode = "f3c6";
    var svgPathData = "M118.3 238.4c3.5-12.5 6.9-33.6 13.2-33.6 8.3 1.8 9.6 23.4 18.6 36.6 4.6-23.5 5.3-85.1 14.1-86.7 9-.7 19.7 66.5 22 77.5 9.9 4.1 48.9 6.6 48.9 6.6 1.9 7.3-24 7.6-40 7.8-4.6 14.8-5.4 27.7-11.4 28-4.7 .2-8.2-28.8-17.5-49.6l-9.4 65.5c-4.4 13-15.5-22.5-21.9-39.3-3.3-.1-62.4-1.6-47.6-7.8l31-5zM228 448c21.2 0 21.2-32 0-32L92 416c-21.2 0-21.2 32 0 32l136 0zm-24 64c21.2 0 21.2-32 0-32l-88 0c-21.2 0-21.2 32 0 32l88 0zm34.2-141.5c3.2-18.9 5.2-36.4 11.9-48.8 7.9-14.7 16.1-28.1 24-41 24.6-40.4 45.9-75.2 45.9-125.5 0-85.6-71.8-155.2-160-155.2S0 69.6 0 155.2c0 50.2 21.3 85.1 45.9 125.5 7.9 12.9 16 26.3 24 41 6.7 12.5 8.7 29.8 11.9 48.9 3.5 21 36.1 15.7 32.6-5.1-3.6-21.7-5.6-40.7-15.3-58.6-32.6-60.4-66.1-95.6-66.1-151.7 0-67.9 57-123.2 127-123.2S287 87.3 287 155.2c0 56.1-33.5 91.3-66.1 151.6-9.7 18-11.7 37.4-15.3 58.6-3.4 20.6 29 26.4 32.6 5.1z";
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
    exports.faMedapps = exports.definition;
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

// src/_entries/faMedapps.js
var import_faMedapps = __toESM(require_faMedapps());
var export_faMedapps = import_faMedapps.definition;
export {
  export_faMedapps as faMedapps
};
