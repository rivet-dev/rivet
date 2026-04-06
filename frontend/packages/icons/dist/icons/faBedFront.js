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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBedFront.js
var require_faBedFront = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBedFront.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "bed-front";
    var width = 512;
    var height = 512;
    var aliases = ["bed-alt"];
    var unicode = "f8f7";
    var svgPathData = "M32 80c0-26.5 21.5-48 48-48l352 0c26.5 0 48 21.5 48 48l0 129.1c-14.5-8.4-30.7-14-48-16.1c0-.3 0-.7 0-1l0-32c0-17.7-14.3-32-32-32l-96 0c-17.7 0-32 14.3-32 32l0 32-32 0 0-32c0-17.7-14.3-32-32-32l-96 0c-17.7 0-32 14.3-32 32l0 32c0 .3 0 .7 0 1c-17.3 2.2-33.6 7.8-48 16.1L32 80zM0 320c0-53 43-96 96-96l320 0c53 0 96 43 96 96l0 128c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-32L64 416l0 32c0 17.7-14.3 32-32 32s-32-14.3-32-32L0 320z";
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
    exports.faBedFront = exports.definition;
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

// src/_entries/faBedFront.js
var import_faBedFront = __toESM(require_faBedFront());
var export_faBedFront = import_faBedFront.definition;
export {
  export_faBedFront as faBedFront
};
