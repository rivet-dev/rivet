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

// src/node_modules/@fortawesome/free-solid-svg-icons/faPlantWilt.js
var require_faPlantWilt = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faPlantWilt.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "plant-wilt";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e5aa";
    var svgPathData = "M344-32c66.3 0 120 53.7 120 120l0 6.2c29.3 12.2 48 41.3 48 76.9 0 27.9-25.3 74.8-66 111.7-3.8 3.5-8.8 5.3-14 5.3s-10.2-1.8-14-5.3c-40.7-36.8-66-83.7-66-111.7 0-35.6 18.7-64.7 48-76.9l0-6.2c0-30.9-25.1-56-56-56s-56 25.1-56 56l0 424c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-296c0-30.9-25.1-56-56-56s-56 25.1-56 56l0 38.2c29.3 12.2 48 41.3 48 76.9 0 27.9-25.3 74.8-66 111.7-3.8 3.5-8.8 5.3-14 5.3s-10.2-1.8-14-5.3C25.3 405.9 0 359 0 331.1 0 295.4 18.7 266.4 48 254.2L48 216c0-66.3 53.7-120 120-120 20.2 0 39.3 5 56 13.8L224 88c0-66.3 53.7-120 120-120z";
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
    exports.faPlantWilt = exports.definition;
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

// src/_entries/faPlantWilt.js
var import_faPlantWilt = __toESM(require_faPlantWilt());
var export_faPlantWilt = import_faPlantWilt.definition;
export {
  export_faPlantWilt as faPlantWilt
};
