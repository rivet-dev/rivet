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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPancakes.js
var require_faPancakes = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPancakes.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "pancakes";
    var width = 512;
    var height = 512;
    var aliases = [129374];
    var unicode = "e42d";
    var svgPathData = "M256 192c0-17.7 14.3-31.9 32-32.6c108.6-4.4 192-31.1 192-63.4c0-35.3-100.3-64-224-64S32 60.7 32 96c0 13.8 15.3 26.6 41.3 37C86.5 138.3 96 150.6 96 164.8L96 272c0 17.7 14.3 32 32 32s32-14.3 32-32l0-100.3c0-9.5 8.1-16.8 17.5-15.7c8.1 .9 14.5 7.6 14.5 15.7l0 52.3c0 17.7 14.3 32 32 32s32-14.3 32-32l0-32zM192 80c0-8.8 7.2-16 16-16l96 0c8.8 0 16 7.2 16 16s-7.2 16-16 16l-96 0c-8.8 0-16-7.2-16-16zM14.9 133C5.3 141.4 0 150.5 0 160c0 20.3 24.2 38.8 64 52.9l0-48.1c0-.1-.1-.2-.2-.4c-.4-.5-1.2-1.3-2.4-1.8c-14.7-5.9-28.7-13.4-39.6-22.9c-2.3-2-4.6-4.3-6.8-6.8zM286.1 239.5C413.3 234.8 512 201 512 160c0-9.5-5.3-18.6-14.9-27c-4 4.5-8.5 8.4-12.7 11.5c-13.4 10-30.9 17.8-49.8 24C396.6 181 345.5 189 289.3 191.3c-.5 0-.9 .2-1.2 .5c-.1 .1-.1 .2-.1 .2l0 32c0 5.3-.7 10.5-1.9 15.5zM7.2 221.1C2.5 227.2 0 233.5 0 240c0 21 25.8 40 68 54.3c-2.6-6.9-4-14.5-4-22.3l0-25.3c-21.5-6.8-41.1-15.2-56.8-25.6zM256 320c141.4 0 256-35.8 256-80c0-6.5-2.5-12.8-7.2-18.9c-16.1 10.6-36.2 19.1-58.3 26c-47.8 14.9-111.2 24-180.2 24.8c-11.3 10-26.1 16.1-42.4 16.1c-11.8 0-22.9-3.2-32.4-8.8c-1.6 14.1-7.8 26.9-17 36.7c25.6 2.7 52.9 4.1 81.4 4.1zm256 0c0-6.5-2.5-12.8-7.2-18.9c-16.1 10.6-36.2 19.1-58.3 26C396.4 342.8 329 352 256 352s-140.4-9.2-190.6-24.9c-22-6.9-42.2-15.4-58.3-26C2.5 307.2 0 313.5 0 320c0 44.2 114.6 80 256 80s256-35.8 256-80zm0 80c0-6.5-2.5-12.8-7.2-18.9c-16.1 10.6-36.2 19.1-58.3 26C396.4 422.8 329 432 256 432s-140.4-9.2-190.6-24.9c-22-6.9-42.2-15.4-58.3-26C2.5 387.2 0 393.5 0 400c0 44.2 114.6 80 256 80s256-35.8 256-80z";
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
    exports.faPancakes = exports.definition;
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

// src/_entries/faPancakes.js
var import_faPancakes = __toESM(require_faPancakes());
var export_faPancakes = import_faPancakes.definition;
export {
  export_faPancakes as faPancakes
};
