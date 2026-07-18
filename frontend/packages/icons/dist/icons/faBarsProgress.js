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

// src/node_modules/@fortawesome/free-solid-svg-icons/faBarsProgress.js
var require_faBarsProgress = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faBarsProgress.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "bars-progress";
    var width = 448;
    var height = 512;
    var aliases = ["tasks-alt"];
    var unicode = "f828";
    var svgPathData = "M288 160l96 0 0-64-96 0 0 64zM0 160L0 80C0 53.5 21.5 32 48 32l352 0c26.5 0 48 21.5 48 48l0 96c0 26.5-21.5 48-48 48L48 224c-26.5 0-48-21.5-48-48l0-16zM160 416l224 0 0-64-224 0 0 64zM0 416l0-80c0-26.5 21.5-48 48-48l352 0c26.5 0 48 21.5 48 48l0 96c0 26.5-21.5 48-48 48L48 480c-26.5 0-48-21.5-48-48l0-16z";
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
    exports.faBarsProgress = exports.definition;
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

// src/_entries/faBarsProgress.js
var import_faBarsProgress = __toESM(require_faBarsProgress());
var export_faBarsProgress = import_faBarsProgress.definition;
export {
  export_faBarsProgress as faBarsProgress
};
