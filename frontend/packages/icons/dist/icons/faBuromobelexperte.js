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

// src/node_modules/@fortawesome/free-brands-svg-icons/faBuromobelexperte.js
var require_faBuromobelexperte = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faBuromobelexperte.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "buromobelexperte";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f37f";
    var svgPathData = "M0 32l0 128 128 0 0-128-128 0zM120 152l-112 0 0-112 112 0 0 112zM160 32l0 128 128 0 0-128-128 0zM280 152l-112 0 0-112 112 0 0 112zM320 32l0 128 128 0 0-128-128 0zM440 152l-112 0 0-112 112 0 0 112zM0 192l0 128 128 0 0-128-128 0zM120 312l-112 0 0-112 112 0 0 112zm40-120l0 128 128 0 0-128-128 0zM280 312l-112 0 0-112 112 0 0 112zm40-120l0 128 128 0 0-128-128 0zM440 312l-112 0 0-112 112 0 0 112zM0 352l0 128 128 0 0-128-128 0zM120 472l-112 0 0-112 112 0 0 112zm40-120l0 128 128 0 0-128-128 0zM280 472l-112 0 0-112 112 0 0 112zm40-120l0 128 128 0 0-128-128 0z";
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
    exports.faBuromobelexperte = exports.definition;
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

// src/_entries/faBuromobelexperte.js
var import_faBuromobelexperte = __toESM(require_faBuromobelexperte());
var export_faBuromobelexperte = import_faBuromobelexperte.definition;
export {
  export_faBuromobelexperte as faBuromobelexperte
};
