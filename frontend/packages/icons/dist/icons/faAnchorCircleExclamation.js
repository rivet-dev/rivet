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

// src/node_modules/@fortawesome/free-solid-svg-icons/faAnchorCircleExclamation.js
var require_faAnchorCircleExclamation = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faAnchorCircleExclamation.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "anchor-circle-exclamation";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e4ab";
    var svgPathData = "M320.5 96a32 32 0 1 1 -64 0 32 32 0 1 1 64 0zm-32-96c-53 0-96 43-96 96 0 41.8 26.7 77.4 64 90.5l0 257.9c-62.9-14.3-110.2-69.7-111.9-136.5l16.1 14.1c10 8.7 25.1 7.7 33.9-2.3s7.7-25.1-2.3-33.9l-64-56c-9-7.9-22.6-7.9-31.6 0l-64 56c-10 8.7-11 23.9-2.3 33.9s23.9 11 33.9 2.3l16.2-14.2c2.1 113.1 94.4 204.1 208 204.1 16.6 0 32.7-1.9 48.2-5.6-20.3-30.5-32.2-67-32.2-106.4 0-27.3 5.7-53.3 16-76.9l0-136.6c37.3-13.2 64-48.7 64-90.5 0-53-43-96-96-96zm208 544a144 144 0 1 0 0-288 144 144 0 1 0 0 288zm0-100a20 20 0 1 1 0 40 20 20 0 1 1 0-40zm0-140c8.8 0 16 7.2 16 16l0 80c0 8.8-7.2 16-16 16s-16-7.2-16-16l0-80c0-8.8 7.2-16 16-16z";
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
    exports.faAnchorCircleExclamation = exports.definition;
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

// src/_entries/faAnchorCircleExclamation.js
var import_faAnchorCircleExclamation = __toESM(require_faAnchorCircleExclamation());
var export_faAnchorCircleExclamation = import_faAnchorCircleExclamation.definition;
export {
  export_faAnchorCircleExclamation as faAnchorCircleExclamation
};
