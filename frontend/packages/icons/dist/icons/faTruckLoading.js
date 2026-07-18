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

// src/node_modules/@fortawesome/free-solid-svg-icons/faTruckRampBox.js
var require_faTruckRampBox = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faTruckRampBox.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "truck-ramp-box";
    var width = 576;
    var height = 512;
    var aliases = ["truck-loading"];
    var unicode = "f4de";
    var svgPathData = "M400 32c-44.2 0-80 35.8-80 80l0 248.2-297.2 89.2C5.9 454.4-3.7 472.3 1.4 489.2s22.9 26.5 39.8 21.5l315.5-94.6 43.7 0c-.2 2.6-.4 5.3-.4 8 0 48.6 39.4 88 88 88s88-39.4 88-88l0-392-176 0zM528 424c0 22.1-17.9 40-40 40s-40-17.9-40-40 17.9-40 40-40c22.1 0 39.9 17.9 40 39.9l0 .1zM51.9 149.5C18.1 159.8-.9 195.6 9.4 229.4l28.1 91.8C47.8 355 83.6 374 117.4 363.7l91.8-28.1c33.8-10.3 52.8-46.1 42.5-79.9l-28.1-91.8c-10.3-33.8-46.1-52.8-79.9-42.5L51.9 149.5z";
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
    exports.faTruckRampBox = exports.definition;
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

// src/_entries/faTruckLoading.js
var import_faTruckRampBox = __toESM(require_faTruckRampBox());
var export_faTruckLoading = import_faTruckRampBox.definition;
export {
  export_faTruckLoading as faTruckLoading
};
