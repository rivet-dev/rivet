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

// src/node_modules/@fortawesome/free-solid-svg-icons/faBorderNone.js
var require_faBorderNone = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faBorderNone.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "border-none";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f850";
    var svgPathData = "M32 416c-17.7 0-32 14.3-32 32s14.3 32 32 32 32-14.3 32-32-14.3-32-32-32zM160 32a32 32 0 1 0 0 64 32 32 0 1 0 0-64zM288 416c-17.7 0-32 14.3-32 32s14.3 32 32 32 32-14.3 32-32-14.3-32-32-32zM160 480c17.7 0 32-14.3 32-32s-14.3-32-32-32-32 14.3-32 32 14.3 32 32 32zM288 32a32 32 0 1 0 0 64 32 32 0 1 0 0-64zM416 480c17.7 0 32-14.3 32-32s-14.3-32-32-32-32 14.3-32 32 14.3 32 32 32zm0-384a32 32 0 1 0 0-64 32 32 0 1 0 0 64zM32 32a32 32 0 1 0 0 64 32 32 0 1 0 0-64zM416 352a32 32 0 1 0 0-64 32 32 0 1 0 0 64zM32 160a32 32 0 1 0 0 64 32 32 0 1 0 0-64zm0 192a32 32 0 1 0 0-64 32 32 0 1 0 0 64zM416 160a32 32 0 1 0 0 64 32 32 0 1 0 0-64z";
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
    exports.faBorderNone = exports.definition;
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

// src/_entries/faBorderNone.js
var import_faBorderNone = __toESM(require_faBorderNone());
var export_faBorderNone = import_faBorderNone.definition;
export {
  export_faBorderNone as faBorderNone
};
