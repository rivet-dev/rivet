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

// src/node_modules/@fortawesome/free-solid-svg-icons/faBackwardFast.js
var require_faBackwardFast = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faBackwardFast.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "backward-fast";
    var width = 512;
    var height = 512;
    var aliases = [9198, "fast-backward"];
    var unicode = "f049";
    var svgPathData = "M492.2 477.6c-12 5-25.7 2.2-34.9-6.9L288 301.3 288 448c0 12.9-7.8 24.6-19.8 29.6s-25.7 2.2-34.9-6.9L64 301.3 64 448c0 17.7-14.3 32-32 32S0 465.7 0 448L0 64C0 46.3 14.3 32 32 32S64 46.3 64 64L64 210.7 233.4 41.4c9.2-9.2 22.9-11.9 34.9-6.9S288 51.1 288 64l0 146.7 169.4-169.4c9.2-9.2 22.9-11.9 34.9-6.9S512 51.1 512 64l0 384c0 12.9-7.8 24.6-19.8 29.6z";
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
    exports.faBackwardFast = exports.definition;
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

// src/_entries/faBackwardFast.js
var import_faBackwardFast = __toESM(require_faBackwardFast());
var export_faBackwardFast = import_faBackwardFast.definition;
export {
  export_faBackwardFast as faBackwardFast
};
