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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faShutters.js
var require_faShutters = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faShutters.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "shutters";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e449";
    var svgPathData = "M32 32C14.3 32 0 46.3 0 64S14.3 96 32 96l448 0c17.7 0 32-14.3 32-32s-14.3-32-32-32L32 32zM20.6 224l470.7 0c11.4 0 20.6-9.2 20.6-20.6c0-2.2-.4-4.4-1.1-6.5L488 128 24 128 1.1 196.8c-.7 2.1-1.1 4.3-1.1 6.5C0 214.8 9.2 224 20.6 224zM491.4 352c11.4 0 20.6-9.2 20.6-20.6c0-2.2-.4-4.4-1.1-6.5L488 256 24 256 1.1 324.8c-.7 2.1-1.1 4.3-1.1 6.5C0 342.8 9.2 352 20.6 352l470.7 0zM20.6 480l470.7 0c11.4 0 20.6-9.2 20.6-20.6c0-2.2-.4-4.4-1.1-6.5L488 384 24 384 1.1 452.8c-.7 2.1-1.1 4.3-1.1 6.5C0 470.8 9.2 480 20.6 480z";
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
    exports.faShutters = exports.definition;
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

// src/_entries/faShutters.js
var import_faShutters = __toESM(require_faShutters());
var export_faShutters = import_faShutters.definition;
export {
  export_faShutters as faShutters
};
