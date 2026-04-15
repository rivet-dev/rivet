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

// src/node_modules/@fortawesome/free-solid-svg-icons/faPlaneSlash.js
var require_faPlaneSlash = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faPlaneSlash.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "plane-slash";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e069";
    var svgPathData = "M41-24.9c-9.4-9.4-24.6-9.4-33.9 0S-2.3-.3 7 9.1l528 528c9.4 9.4 24.6 9.4 33.9 0s9.4-24.6 0-33.9l-183.4-183.4 7.1-7.7 127.3 0c30.9 0 56-25.1 56-56s-25.1-56-56-56L392.7 200 233.5 26.4C227.5 19.8 218.9 16 209.9 16l-43.7 0c-10.9 0-18.6 10.7-15.2 21.1L175.1 109.2 41-24.9zM130.2 200l-24.6 0-52.8-66c-3-3.8-7.6-6-12.5-6l-19.8 0c-10.4 0-18 9.8-15.5 19.9L32 256 5 364.1C2.4 374.2 10.1 384 20.5 384l19.8 0c4.9 0 9.5-2.2 12.5-6l52.8-66 99.7 0-54.3 162.9c-3.5 10.4 4.3 21.1 15.2 21.1l43.7 0c9 0 17.5-3.8 23.6-10.4L320.7 390.5 130.2 200z";
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
    exports.faPlaneSlash = exports.definition;
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

// src/_entries/faPlaneSlash.js
var import_faPlaneSlash = __toESM(require_faPlaneSlash());
var export_faPlaneSlash = import_faPlaneSlash.definition;
export {
  export_faPlaneSlash as faPlaneSlash
};
