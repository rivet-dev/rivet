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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faHockeyMask.js
var require_faHockeyMask = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faHockeyMask.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "hockey-mask";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f6ee";
    var svgPathData = "M0 240C0 111 64 0 224 0S448 111 448 240c0 192-48 272-224 272S0 432 0 240zM160 104a24 24 0 1 0 0-48 24 24 0 1 0 0 48zm88 24a24 24 0 1 0 -48 0 24 24 0 1 0 48 0zM176 344a24 24 0 1 0 0-48 24 24 0 1 0 0 48zm0 64a24 24 0 1 0 0-48 24 24 0 1 0 0 48zm24 40a24 24 0 1 0 -48 0 24 24 0 1 0 48 0zm72-104a24 24 0 1 0 0-48 24 24 0 1 0 0 48zM312 80a24 24 0 1 0 -48 0 24 24 0 1 0 48 0zM272 408a24 24 0 1 0 0-48 24 24 0 1 0 0 48zm24 40a24 24 0 1 0 -48 0 24 24 0 1 0 48 0zM128 272c35.3 0 64-28.7 64-64c0-17.7-14.3-32-32-32l-64 0c-17.7 0-32 14.3-32 32c0 35.3 28.7 64 64 64zm256-64c0-17.7-14.3-32-32-32l-64 0c-17.7 0-32 14.3-32 32c0 35.3 28.7 64 64 64s64-28.7 64-64z";
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
    exports.faHockeyMask = exports.definition;
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

// src/_entries/faHockeyMask.js
var import_faHockeyMask = __toESM(require_faHockeyMask());
var export_faHockeyMask = import_faHockeyMask.definition;
export {
  export_faHockeyMask as faHockeyMask
};
