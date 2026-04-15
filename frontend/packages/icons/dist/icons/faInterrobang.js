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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faInterrobang.js
var require_faInterrobang = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faInterrobang.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "interrobang";
    var width = 320;
    var height = 512;
    var aliases = [8253];
    var unicode = "e5ba";
    var svgPathData = "M144 96c-35.3 0-64 28.7-64 64c0 17.7-14.3 32-32 32s-32-14.3-32-32C16 89.3 73.3 32 144 32l32 0c70.7 0 128 57.3 128 128l0 3.6c0 43.6-22.1 84.1-58.8 107.7L203 298.4c-6.9 4.4-11 12-11 20.2l0 1.4c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-1.4L128 176c0-17.7 14.3-32 32-32s32 14.3 32 32l0 53.4 18.6-12c18.3-11.8 29.4-32.1 29.4-53.8l0-3.6c0-35.3-28.7-64-64-64l-32 0zM120 440a40 40 0 1 1 80 0 40 40 0 1 1 -80 0z";
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
    exports.faInterrobang = exports.definition;
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

// src/_entries/faInterrobang.js
var import_faInterrobang = __toESM(require_faInterrobang());
var export_faInterrobang = import_faInterrobang.definition;
export {
  export_faInterrobang as faInterrobang
};
