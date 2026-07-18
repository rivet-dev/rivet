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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBroccoli.js
var require_faBroccoli = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBroccoli.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "broccoli";
    var width = 512;
    var height = 512;
    var aliases = [129382];
    var unicode = "e3e2";
    var svgPathData = "M0 256c0 53 43 96 96 96l320 0c53 0 96-43 96-96c0-45.5-31.6-83.6-74.1-93.5c6.4-10 10.1-21.8 10.1-34.5c0-35.3-28.7-64-64-64c-12.7 0-24.6 3.7-34.5 10.1C339.6 31.6 301.5 0 256 0s-83.6 31.6-93.5 74.1C152.6 67.7 140.7 64 128 64c-35.3 0-64 28.7-64 64c0 12.7 3.7 24.6 10.1 34.5C31.6 172.4 0 210.5 0 256zM192.2 490.3c5 12.8 17.1 21.7 30.9 21.7l65.9 0c13.8 0 25.9-8.9 30.9-21.7c17.6-45.3 39.6-82.1 56.1-106.3L280 384l-16.8 33.7c-2.9 5.9-11.4 5.9-14.3 0L232 384l-95.9 0c16.5 24.2 38.5 60.9 56.1 106.3z";
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
    exports.faBroccoli = exports.definition;
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

// src/_entries/faBroccoli.js
var import_faBroccoli = __toESM(require_faBroccoli());
var export_faBroccoli = import_faBroccoli.definition;
export {
  export_faBroccoli as faBroccoli
};
