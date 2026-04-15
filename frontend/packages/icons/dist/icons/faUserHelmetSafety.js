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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faUserHelmetSafety.js
var require_faUserHelmetSafety = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faUserHelmetSafety.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "user-helmet-safety";
    var width = 448;
    var height = 512;
    var aliases = [128119, "user-construction", "user-hard-hat"];
    var unicode = "f82c";
    var svgPathData = "M216 0c-13.3 0-24 10.7-24 24l0 56L163.6 23.1C125.6 43.5 99 82.6 96.2 128l-.2 0c-13.3 0-24 10.7-24 24s10.7 24 24 24l256 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-.2 0C349 82.6 322.4 43.5 284.4 23.1L256 80l0-56c0-13.3-10.7-24-24-24L216 0zm8 320c65.3 0 119.1-48.9 127-112L97 208c7.9 63.1 61.7 112 127 112zm-89.6 32C60.2 352 0 412.2 0 486.4C0 500.5 11.5 512 25.6 512l396.8 0c14.1 0 25.6-11.5 25.6-25.6C448 412.2 387.8 352 313.6 352l-179.2 0z";
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
    exports.faUserHelmetSafety = exports.definition;
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

// src/_entries/faUserHelmetSafety.js
var import_faUserHelmetSafety = __toESM(require_faUserHelmetSafety());
var export_faUserHelmetSafety = import_faUserHelmetSafety.definition;
export {
  export_faUserHelmetSafety as faUserHelmetSafety
};
