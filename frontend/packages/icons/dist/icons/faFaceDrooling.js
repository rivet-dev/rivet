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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceDrooling.js
var require_faFaceDrooling = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceDrooling.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "face-drooling";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e372";
    var svgPathData = "M384 477.8c76.5-44.3 128-127 128-221.8C512 114.6 397.4 0 256 0S0 114.6 0 256S114.6 512 256 512c33.9 0 66.4-6.6 96-18.6l0-122.4c-9.7 6.4-20.3 12.2-32 16.9l0 60.2c0 8.8-7.2 16-16 16s-16-7.2-16-16l0-50.9c-10.1 1.9-20.8 2.9-32 2.9c-67 0-113.8-36.7-140.5-70c-5.5-6.9-4.4-17 2.5-22.5s17-4.4 22.5 2.5c22.8 28.5 61.3 58 115.5 58s92.7-29.5 115.5-58c5.5-6.9 15.6-8 22.5-2.5s8 15.6 2.5 22.5c-3.8 4.7-7.9 9.5-12.5 14.2l0 133.6zM158.6 189.6c-8.4 8-13.5 17.7-15.4 23.5c-2.8 8.4-11.9 12.9-20.2 10.1s-12.9-11.9-10.1-20.2c3.4-10.2 11.1-24.6 23.8-36.5C149.5 154.2 167.9 144 192 144c8.8 0 16 7.2 16 16s-7.2 16-16 16c-14.3 0-25.2 5.8-33.4 13.6zm210.2 23.5c-1.9-5.8-7-15.4-15.4-23.5c-8.2-7.8-19-13.6-33.4-13.6c-8.8 0-16-7.2-16-16s7.2-16 16-16c24.1 0 42.5 10.2 55.4 22.4c12.6 12 20.4 26.3 23.8 36.5c2.8 8.4-1.7 17.4-10.1 20.2s-17.4-1.7-20.2-10.1z";
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
    exports.faFaceDrooling = exports.definition;
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

// src/_entries/faFaceDrooling.js
var import_faFaceDrooling = __toESM(require_faFaceDrooling());
var export_faFaceDrooling = import_faFaceDrooling.definition;
export {
  export_faFaceDrooling as faFaceDrooling
};
