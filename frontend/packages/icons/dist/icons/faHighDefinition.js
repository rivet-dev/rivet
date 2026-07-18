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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faHighDefinition.js
var require_faHighDefinition = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faHighDefinition.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "high-definition";
    var width = 576;
    var height = 512;
    var aliases = ["rectangle-hd"];
    var unicode = "e1ae";
    var svgPathData = "M0 96C0 60.7 28.7 32 64 32l448 0c35.3 0 64 28.7 64 64l0 320c0 35.3-28.7 64-64 64L64 480c-35.3 0-64-28.7-64-64L0 96zm304 88l0 144c0 13.3 10.7 24 24 24l56 0c53 0 96-43 96-96s-43-96-96-96l-56 0c-13.3 0-24 10.7-24 24zm80 24c26.5 0 48 21.5 48 48s-21.5 48-48 48l-32 0 0-96 32 0zM160 184c0-13.3-10.7-24-24-24s-24 10.7-24 24l0 72 0 72c0 13.3 10.7 24 24 24s24-10.7 24-24l0-48 64 0 0 48c0 13.3 10.7 24 24 24s24-10.7 24-24l0-72 0-72c0-13.3-10.7-24-24-24s-24 10.7-24 24l0 48-64 0 0-48z";
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
    exports.faHighDefinition = exports.definition;
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

// src/_entries/faHighDefinition.js
var import_faHighDefinition = __toESM(require_faHighDefinition());
var export_faHighDefinition = import_faHighDefinition.definition;
export {
  export_faHighDefinition as faHighDefinition
};
