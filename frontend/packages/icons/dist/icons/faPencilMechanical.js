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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPencilMechanical.js
var require_faPencilMechanical = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPencilMechanical.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "pencil-mechanical";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e5ca";
    var svgPathData = "M475.3 11.3C468.1 4.1 458.2 0 448 0s-20.1 4.1-27.3 11.3L390.6 41.4l80 80 30.1-30.1C507.9 84.1 512 74.2 512 64s-4.1-20.1-11.3-27.3L475.3 11.3zm-233 76.3c3.1-3.1 8.2-3.1 11.3 0L283 117 72.8 327.2C67 333 62.8 340.1 60.6 348L32.8 445.3 7 471c-9.4 9.4-9.4 24.6 0 33.9s24.6 9.4 33.9 0l25.7-25.7L164 451.4c7.8-2.2 15-6.4 20.8-12.2L436.7 187.3c7.2-7.2 11.3-17.1 11.3-27.3s-4.1-20.1-11.3-27.3L379.3 75.3C372.1 68.1 362.2 64 352 64s-20.1 4.1-27.3 11.3L317 83 287.6 53.7c-21.9-21.9-57.3-21.9-79.2 0L103 159c-9.4 9.4-9.4 24.6 0 33.9s24.6 9.4 33.9 0L242.3 87.6zM89.1 422.9l17.3-60.5 43.2 43.2L89.1 422.9z";
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
    exports.faPencilMechanical = exports.definition;
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

// src/_entries/faPencilMechanical.js
var import_faPencilMechanical = __toESM(require_faPencilMechanical());
var export_faPencilMechanical = import_faPencilMechanical.definition;
export {
  export_faPencilMechanical as faPencilMechanical
};
