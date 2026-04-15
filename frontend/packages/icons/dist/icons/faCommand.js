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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCommand.js
var require_faCommand = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCommand.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "command";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e142";
    var svgPathData = "M0 96C0 43 43 0 96 0s96 43 96 96l0 32 128 0 0-32c0-53 43-96 96-96s96 43 96 96s-43 96-96 96l-32 0 0 128 32 0c53 0 96 43 96 96s-43 96-96 96s-96-43-96-96l0-32-128 0 0 32c0 53-43 96-96 96s-96-43-96-96s43-96 96-96l32 0 0-128-32 0C43 192 0 149 0 96zm128 32l0-32c0-17.7-14.3-32-32-32S64 78.3 64 96s14.3 32 32 32l32 0zm64 192l128 0 0-128-128 0 0 128zm-64 64l-32 0c-17.7 0-32 14.3-32 32s14.3 32 32 32s32-14.3 32-32l0-32zm256 0l0 32c0 17.7 14.3 32 32 32s32-14.3 32-32s-14.3-32-32-32l-32 0zm0-256l32 0c17.7 0 32-14.3 32-32s-14.3-32-32-32s-32 14.3-32 32l0 32z";
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
    exports.faCommand = exports.definition;
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

// src/_entries/faCommand.js
var import_faCommand = __toESM(require_faCommand());
var export_faCommand = import_faCommand.definition;
export {
  export_faCommand as faCommand
};
