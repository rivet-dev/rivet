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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCarMirrors.js
var require_faCarMirrors = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCarMirrors.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "car-mirrors";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e343";
    var svgPathData = "M167.2 117.4c4.5-12.8 16.6-21.4 30.2-21.4l181.2 0c13.6 0 25.7 8.6 30.2 21.4L434.9 192l-293.8 0 26.1-74.6zM32 224l8.6 0c-5.4 9.4-8.6 20.3-8.6 32l0 144 0 48c0 17.7 14.3 32 32 32l32 0c17.7 0 32-14.3 32-32l0-48 320 0 0 48c0 17.7 14.3 32 32 32l32 0c17.7 0 32-14.3 32-32l0-48 0-144c0-11.7-3.1-22.6-8.6-32l8.6 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-52.5 0L469.2 96.3C455.7 57.8 419.4 32 378.6 32L197.4 32c-40.8 0-77.1 25.8-90.6 64.3L84.5 160 32 160c-17.7 0-32 14.3-32 32s14.3 32 32 32zm64 64a32 32 0 1 1 64 0 32 32 0 1 1 -64 0zm352-32a32 32 0 1 1 0 64 32 32 0 1 1 0-64z";
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
    exports.faCarMirrors = exports.definition;
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

// src/_entries/faCarMirrors.js
var import_faCarMirrors = __toESM(require_faCarMirrors());
var export_faCarMirrors = import_faCarMirrors.definition;
export {
  export_faCarMirrors as faCarMirrors
};
