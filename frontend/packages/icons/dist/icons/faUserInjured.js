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

// src/node_modules/@fortawesome/free-solid-svg-icons/faUserInjured.js
var require_faUserInjured = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faUserInjured.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "user-injured";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f728";
    var svgPathData = "M242.7 80L334 80c-10.7-24.6-29.5-44.9-52.9-57.6L242.7 80zm-.9-70.7C236 8.4 230 8 224 8 174.8 8 132.5 37.6 114 80l80.6 0 47.1-70.7zM224 248c66.3 0 120-53.7 120-120l-240 0c0 66.3 53.7 120 120 120zM98.7 341.8C49.3 370.2 16 423.5 16 484.6 16 499.7 28.3 512 43.4 512l151 0-95.8-170.2zm45.1-17.7l42.7 75.9 85.5 0c44.2 0 80 35.8 80 80 0 11.4-2.4 22.2-6.7 32l59.2 0c15.1 0 27.4-12.3 27.4-27.4 0-90.9-73.7-164.6-164.6-164.6l-86.9 0c-12.6 0-24.9 1.4-36.7 4.1zM213.5 448l36 64 22.5 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-58.5 0z";
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
    exports.faUserInjured = exports.definition;
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

// src/_entries/faUserInjured.js
var import_faUserInjured = __toESM(require_faUserInjured());
var export_faUserInjured = import_faUserInjured.definition;
export {
  export_faUserInjured as faUserInjured
};
