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

// src/node_modules/@fortawesome/free-brands-svg-icons/faDeploydog.js
var require_faDeploydog = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faDeploydog.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "deploydog";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f38e";
    var svgPathData = "M382.2 136l51.7 0 0 239.6-51.7 0 0-20.7c-19.8 24.8-52.8 24.1-73.8 14.7-26.2-11.7-44.3-38.1-44.3-71.8 0-29.8 14.8-57.9 43.3-70.8 20.2-9.1 52.7-10.6 74.8 12.9l0-103.9zM317.5 297.8a33.2 33.2 0 1 0 66.4 1 33.2 33.2 0 1 0 -66.4-1zM188.5 136l51.7 0 0 239.6-51.7 0 0-20.7c-19.8 24.8-52.8 24.1-73.8 14.7-26.2-11.7-44.3-38.1-44.3-71.8 0-29.8 14.8-57.9 43.3-70.8 20.2-9.1 52.7-10.6 74.8 12.9l0-103.9zM123.8 297.8a33.2 33.2 0 1 0 66.4 1 33.2 33.2 0 1 0 -66.4-1zM448 96c17.5 0 32 14.4 32 32l0 256c0 17.5-14.4 32-32 32L64 416c-17.5 0-32-14.4-32-32l0-256c0-17.5 14.4-32 32-32l384 0zm0-32L64 64C28.8 64 0 92.8 0 128L0 384c0 35.2 28.8 64 64 64l384 0c35.2 0 64-28.8 64-64l0-256c0-35.2-28.8-64-64-64z";
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
    exports.faDeploydog = exports.definition;
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

// src/_entries/faDeploydog.js
var import_faDeploydog = __toESM(require_faDeploydog());
var export_faDeploydog = import_faDeploydog.definition;
export {
  export_faDeploydog as faDeploydog
};
