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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSteak.js
var require_faSteak = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSteak.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "steak";
    var width = 576;
    var height = 512;
    var aliases = [129385];
    var unicode = "f824";
    var svgPathData = "M268.8 37.4C295.7 12.4 329.7 0 368 0c65 0 117.7 39 150.5 80c33.9 42.4 57.5 99.6 57.5 160c0 83.9-55.1 152.8-113.4 196.2C402.1 481.2 321.7 512 240 512c-36.8 0-86.5-4.4-130-19.3C73.4 480.1 0 444.3 0 360c0-38.6 17.8-68.3 42.5-88.1c19.6-15.8 41.4-26.8 63.3-37.8c27.3-13.8 54.6-27.6 77.7-50.7c13.8-13.8 19.7-27.9 30-52.9c3.2-7.7 6.8-16.5 11.2-26.6c7.7-17.6 20.6-44.7 44.1-66.5zm36.8 122s0 0 0 0c2.5-6 4.8-11.8 7.1-17c6.9-15.7 13.3-27.1 21.5-34.7c7.5-7 17.5-11.7 33.9-11.7c25.4 0 53.3 16.2 75.5 44c22 27.5 36.5 64.1 36.5 100c0 40.8-28.3 84.6-74.7 119.2C359.4 393.4 298.6 416 240 416c-31 0-69.1-3.9-98.8-14.1C110.1 391.2 96 376.7 96 360c0-9 2.6-13.2 6.9-16.9c5.8-5 15-9.1 30.2-15.5l1.2-.5c29.3-12.4 71.2-30 117-75.8c28.6-28.6 42.9-63.7 54.3-91.9zm6.8-75.2c-13.3 12.4-21.9 29-29 45.3c-2.9 6.6-5.6 13.2-8.3 19.8c-11.4 27.8-22.8 55.7-46.4 79.3c-41.4 41.4-78.6 57.1-107 69l-1.1 .5-.7 .3s0 0 0 0c-13.5 5.7-27.5 11.6-37.7 20.3C70.4 328.8 64 342.1 64 360c0 39.3 33.9 60.8 66.8 72.1C165.1 443.9 207 448 240 448c66.3 0 133.6-25.4 184.4-63.2C474.8 347.4 512 295.2 512 240c0-44.1-17.5-87.5-43.5-120C442.7 87.8 406.6 64 368 64c-23.7 0-41.7 7.3-55.6 20.3zM384 256a32 32 0 1 0 0-64 32 32 0 1 0 0 64z";
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
    exports.faSteak = exports.definition;
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

// src/_entries/faSteak.js
var import_faSteak = __toESM(require_faSteak());
var export_faSteak = import_faSteak.definition;
export {
  export_faSteak as faSteak
};
