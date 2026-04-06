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

// src/node_modules/@fortawesome/free-solid-svg-icons/faPlusMinus.js
var require_faPlusMinus = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faPlusMinus.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "plus-minus";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "e43c";
    var svgPathData = "M224 32c0-17.7-14.3-32-32-32s-32 14.3-32 32l0 128-128 0c-17.7 0-32 14.3-32 32s14.3 32 32 32l128 0 0 128c0 17.7 14.3 32 32 32s32-14.3 32-32l0-128 128 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-128 0 0-128zM0 480c0 17.7 14.3 32 32 32l320 0c17.7 0 32-14.3 32-32s-14.3-32-32-32L32 448c-17.7 0-32 14.3-32 32z";
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
    exports.faPlusMinus = exports.definition;
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

// src/_entries/faPlusMinus.js
var import_faPlusMinus = __toESM(require_faPlusMinus());
var export_faPlusMinus = import_faPlusMinus.definition;
export {
  export_faPlusMinus as faPlusMinus
};
